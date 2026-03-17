# HiveNode Worker Technical Specification

## Purpose

HiveNode is the worker-side runtime in the Hive system. It connects outbound to HiveCore, exposes local Ollama capacity to the cluster, and manages the local Ollama runtime when configured for Docker-managed mode.

This document describes the worker as implemented in this repository today. It is intended to be a technical reference for maintainers, not a marketing overview.

## Scope

This specification covers:

- Process startup and runtime initialization
- Worker connection lifecycle
- Authentication and polling
- Request proxying to Ollama
- Control commands from HiveCore
- Docker-managed Ollama behavior
- Shared state and refresh behavior
- Error handling and reconnect behavior
- Observability and metrics hooks

This specification does not define HiveCore internals.

## High-Level Architecture

HiveNode is a single process with these major responsibilities:

1. Initialize logging and optional InfluxDB metrics.
2. Initialize the Ollama runtime.
3. Open `CONCURRENT_REQUESTS` parallel worker connections to HiveCore.
4. Authenticate each connection and advertise runtime versions.
5. Poll HiveCore for work.
6. Either:
   - handle a Hive control message, or
   - proxy an HTTP request to local Ollama and stream the response back.
7. Reconnect on connection failure or intentional reboot.

The primary implementation entry points are:

- `src/main.rs`
- `src/protocol/connection.rs`
- `src/protocol/network_util.rs`
- `src/protocol/docker.rs`
- `src/protocol/state.rs`

## Runtime Model

### Process Model

The process starts under a Tokio runtime created by `#[tokio::main]`.

Startup sequence:

1. Set `RUST_LOG`.
2. Initialize structured logging.
3. Load `.env`.
4. Initialize optional Influx logging.
5. Configure Ollama according to `OLLAMA_MODE`.
6. Spawn `CONCURRENT_REQUESTS` OS threads.
7. Run one HiveCore connection loop per thread.

Each worker thread independently calls `run_protocol(nonce)`.

### Concurrency Model

- All worker threads share process-global state through `RwLock`-backed globals.
- Each thread maintains its own TCP connection to HiveCore.
- Each thread owns its own blocking `reqwest::Client` for Ollama proxying.
- Docker upgrade activity is coordinated with `DOCKER_UPGRADE_LOCK`.

## Configuration Contract

### Required Environment

Core connection:

- `HIVE_CORE_URL`
- `HIVE_KEY`
- `CONCURRENT_REQUESTS`

Docker-managed Ollama mode:

- `OLLAMA_MODE=docker` or unset
- `OLLAMA_PORT`
- `HIVE_OLLAMA_MODELS`

External Ollama mode:

- `OLLAMA_MODE=external`
- `OLLAMA_URL`

Optional:

- `GPU_PASSTHROUGH`
- `INFLUX_HOST`
- `INFLUX_ORG`
- `INFLUX_TOKEN`

### Ollama Mode Resolution

`OLLAMA_MODE` is parsed into:

- `docker`
- `external`

Unset defaults to `docker`.

Invalid values are rejected at startup.

## Ollama Runtime Management

### Docker-Managed Mode

In Docker mode, HiveNode is responsible for:

- ensuring the `ollama/ollama:latest` image is present
- starting a named container derived from the worker key
- binding container port `11434/tcp` to host `OLLAMA_PORT`
- mounting `HIVE_OLLAMA_MODELS` into `/root/.ollama`
- configuring optional GPU passthrough
- waiting for `/api/version` to become reachable

Container naming scheme:

- `ollama-hive-<first five chars of HIVE_KEY>`

Once the container is ready, HiveNode sets:

- `OLLAMA_URL=http://127.0.0.1:<OLLAMA_PORT>`

### External Mode

In external mode, HiveNode does not manage Docker. It requires `OLLAMA_URL` and uses that value directly.

### Docker Upgrade Path

The Docker upgrade flow:

1. Pull `ollama/ollama:latest`
2. Acquire the global Docker upgrade write lock
3. Stop the current container if present
4. Remove the old container
5. Recreate the container with the configured mounts, ports, and GPU settings
6. Start the new container
7. Poll `OLLAMA_URL/api/version` until reachable

After successful upgrade, HiveNode:

- calls `notify_refresh()`
- sets the reboot flag

This causes worker threads to reconnect and refresh model state.

## Connection Lifecycle

### Establishing a Connection

Each worker thread:

1. Connects to `HIVE_CORE_URL` over TCP
2. Creates a blocking HTTP client for Ollama communication
3. Refreshes local model metadata under the Docker read lock
4. Authenticates to HiveCore

Authentication payload format:

```text
AUTH <HIVE_KEY>;<nonce>;<node_version>;<ollama_version> HIVE\r\n
```

Where:

- `nonce` is shared across all connections from the same process start
- `node_version` is `CARGO_PKG_VERSION`
- `ollama_version` comes from `GET /api/version`, or `Unknown` on failure

On successful authentication, HiveNode stores the HiveCore-assigned node name in global state.

### Poll Loop

Once authenticated, each thread enters a loop:

1. Compare local model refresh timestamp with the global refresh timestamp
2. Refresh models if needed
3. Send a poll command
4. Read the next message from HiveCore
5. Dispatch the message either to control handling or proxy handling
6. Exit the loop if reboot or shutdown was requested

Polling payload format:

```text
POLL - HIVE\r\n
```

or

```text
POLL <model_target> HIVE\r\n
```

Polling behavior:

- After a model refresh, optimized polling is disabled for one iteration.
- Otherwise HiveNode sends `POLL - HIVE`, which tells HiveCore to reuse the last advertised model set and optimize ordering.

## Message Framing and Parsing

### HiveCore to HiveNode

Inbound messages from HiveCore are length-prefixed:

1. Read a 4-byte big-endian length
2. Read exactly that many bytes
3. Parse the payload as a textual message

The parsed representation is `ProxyMessage` from `src/messages/proxy_message.rs` with fields:

- `protocol`
- `method`
- `uri`
- `headers`
- `body`

### Parsing Rules

The first line is split on whitespace into:

- `method`
- `uri`
- `protocol`

Headers follow as `Name: Value` pairs until a blank line.

The remaining content is the body.

### Worker Command Recognition

The current worker recognizes these control commands:

- `REBOOT`
- `SHUTDOWN`
- `UPDATE`
- `UPDATE_OLLAMA`

For HIVE messages, the command name is taken from the message method.

For HTTP-shaped `/worker/command` messages, the command name is taken from the body.

## Control Flow

All inbound messages with `protocol == "HIVE"` are treated as control-plane messages and routed through the control handler.

### `PONG`

`PONG` is treated as a keepalive/control message and produces no side effects.

### `REBOOT`

Behavior:

- set global reboot flag
- write an HTTP `200 OK` response to the current stream

### `SHUTDOWN`

Behavior:

- set global shutdown flag
- write an HTTP `200 OK` response to the current stream

### `UPDATE` / `UPDATE_OLLAMA`

Behavior:

1. If not in Docker-managed mode:
   - write HTTP `409 Conflict`
   - do nothing else
2. If in Docker-managed mode:
   - write HTTP `202 Accepted`
   - spawn a background thread
   - run the Docker upgrade flow in that thread
   - on success, set refresh and reboot flags

The upgrade itself does not block the control handler after the initial acknowledgement is written.

## Proxy Flow

Messages whose `protocol` is not `HIVE` are treated as proxied Ollama requests.

### Request Forwarding

HiveNode forwards requests to:

- `OLLAMA_URL + request.uri`

Forwarding rules:

- preserve the original HTTP method
- forward most headers
- drop `Host`
- drop `Content-Length`
- preserve the body if present
- apply a 30-minute request timeout

HiveNode refuses to send HIVE protocol requests to Ollama.

### Response Streaming

Ollama responses are written back to HiveCore as HTTP/1.1 chunked transfer encoding:

1. Write HTTP status line
2. Forward response headers except `Transfer-Encoding`
3. Force `Transfer-Encoding: chunked`
4. Force `Connection: close`
5. Stream the body line-by-line as chunks
6. Write final `0\r\n\r\n`

### Poll Refresh Side Effects

Some proxied requests imply model inventory changes and therefore require a model refresh on the next loop.

The current implementation treats these endpoints as refresh-triggering:

- `POST /api/pull`
- `DELETE /api/delete`

When one of those completes successfully, `stream_response_to_proxy()` returns `true`, causing the caller to invoke `notify_refresh()`.

## Model Discovery

Model discovery is performed through Ollama `GET /api/tags`.

The returned tags are converted into a `Poller` model target string. That target is stored per connection and used in polling.

Model refresh happens:

- once during connection startup
- after a successful proxied model-changing request
- after a successful Docker upgrade
- whenever the local thread sees the global refresh timestamp move forward

## Shared State

The worker uses process-global state for:

- last refresh timestamp
- node name
- reboot flag
- shutdown flag

This state is defined in `src/protocol/state.rs`.

### Semantics

- `notify_refresh()` updates the global refresh timestamp to `Utc::now()`
- `set_reboot(true)` requests all worker loops to exit and reconnect
- `set_shutdown(true)` requests all worker loops to exit permanently
- `set_node_name(...)` stores the authenticated node identity

## Locking Behavior

### Docker Upgrade Lock

`DOCKER_UPGRADE_LOCK` is a process-wide `RwLock<()>`.

Read lock use:

- normal model refresh
- normal request proxying

Write lock use:

- destructive Docker upgrade steps

This allows normal operations to proceed concurrently most of the time while preventing proxy/model operations from racing with container replacement.

## Error Handling and Recovery

### Connection-Level Errors

If `run_protocol()` returns an error:

- the worker thread logs the error
- waits 10 seconds
- clears reboot state
- reconnects unless shutdown has been requested

### Message Read Errors

If HiveNode cannot read the next message length or payload from HiveCore, the connection is considered failed and the thread reconnects.

### Ollama Version Lookup

Failure to fetch Ollama version during authentication does not fail startup. The worker reports `Unknown`.

### Docker Upgrade Failures

If a background Docker upgrade fails:

- the failure is logged
- no success refresh/reboot is triggered

The worker process continues running.

## Observability

### Logging

The worker logs:

- startup and configuration events
- authentication results
- control messages
- proxy request lifecycle
- Docker lifecycle events
- failures and reconnects

### InfluxDB

Influx logging is optional. When configured, HiveNode records proxied request success and error events, including:

- model
- protocol
- method
- URI
- status
- response code

The response stream is sanitized for newline removal before submission.

## Implementation Notes

### Mixed Protocol Behavior

The worker currently uses two different outbound formats over the HiveCore TCP stream:

- plain text HIVE commands for worker-initiated actions such as `AUTH` and `POLL`
- HTTP/1.1 response bytes for control acknowledgements and proxied Ollama responses

This behavior is part of the current implementation contract and should be treated carefully when changing protocol logic.

### Current Control-Plane Assumption

The current implementation assumes that HiveCore-issued control commands can be answered by writing HTTP-style responses back to the same worker TCP stream. Any future protocol changes should verify this assumption against HiveCore’s parser and worker-command bridge.

## Sequence Summary

### Normal Startup

1. Process starts
2. Ollama runtime is prepared
3. Worker threads spawn
4. Each thread refreshes models
5. Each thread authenticates
6. Each thread enters poll loop

### Normal Proxy Request

1. HiveCore sends a non-HIVE framed request
2. Worker forwards it to Ollama
3. Worker streams the HTTP response back
4. Worker optionally triggers model refresh if the endpoint changed local models

### Docker Update Command

1. HiveCore sends `UPDATE` or `UPDATE_OLLAMA`
2. Worker writes `202 Accepted`
3. Worker spawns a background upgrade thread
4. Upgrade thread replaces the container
5. Worker marks refresh and reboot
6. Connection loop exits and reconnects

## Non-Goals

HiveNode does not:

- expose a public HTTP server of its own
- persist local job state across restarts
- implement HiveCore queueing logic
- define the HiveCore admin API
