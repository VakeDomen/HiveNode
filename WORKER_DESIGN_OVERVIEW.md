# HiveNode Worker Design Overview

## Purpose

HiveNode is the worker process that connects to HiveCore, polls for work, forwards inference requests to Ollama, and streams results back to HiveCore.

This document is the short version of the worker design. For implementation detail, see `WORKER_TECHNICAL_SPEC.md`.

## What the Worker Does

At a high level, the worker:

1. Starts logging and loads configuration
2. Prepares Ollama
3. Opens multiple parallel connections to HiveCore
4. Authenticates each connection
5. Polls HiveCore for tasks
6. Either handles a control message or proxies an Ollama request
7. Reconnects on failure or reboot

## Main Components

- `src/main.rs`
  Process entry point. Initializes logging, environment, Ollama runtime, and worker threads.

- `src/protocol/connection.rs`
  Owns the per-connection loop: connect, authenticate, poll, dispatch, reconnect.

- `src/protocol/network_util.rs`
  Implements authentication, polling, request parsing, control handling, proxy forwarding, and response streaming.

- `src/protocol/docker.rs`
  Manages Docker-backed Ollama startup and upgrades.

- `src/protocol/state.rs`
  Holds shared process state such as refresh timestamps, node name, reboot, and shutdown flags.

## Runtime Model

HiveNode is one process with multiple worker threads.

- The number of parallel worker connections is controlled by `CONCURRENT_REQUESTS`.
- Each thread has its own TCP connection to HiveCore.
- Each thread can proxy requests independently.
- Shared flags are coordinated through global locks.

This design keeps the runtime simple while allowing multiple concurrent inference streams from one machine.

## Ollama Modes

HiveNode supports two Ollama modes:

- `docker`
  HiveNode manages the Ollama container itself.

- `external`
  HiveNode talks to an already running Ollama instance through `OLLAMA_URL`.

Docker mode is the default.

## Connection Lifecycle

For each worker thread, the connection lifecycle is:

1. Connect to `HIVE_CORE_URL`
2. Refresh local model state
3. Send `AUTH`
4. Receive authenticated identity from HiveCore
5. Enter the poll loop

The loop then repeats:

1. Refresh models if needed
2. Send `POLL`
3. Read one framed message from HiveCore
4. Route it to either:
   - control handling for `HIVE` messages
   - Ollama proxying for non-`HIVE` messages

## Control Messages

The current worker recognizes:

- `PONG`
- `REBOOT`
- `SHUTDOWN`
- `UPDATE`
- `UPDATE_OLLAMA`

Expected behavior:

- `PONG` is a keepalive with no side effects.
- `REBOOT` asks the worker to reconnect.
- `SHUTDOWN` asks the worker to exit.
- `UPDATE` and `UPDATE_OLLAMA` trigger a Docker-managed Ollama upgrade.

## Proxy Flow

For normal inference traffic:

1. HiveCore sends an HTTP-shaped message over the worker TCP stream.
2. HiveNode converts it into a request to `OLLAMA_URL`.
3. Ollama returns a normal HTTP response.
4. HiveNode streams that response back to HiveCore using chunked transfer encoding.

HiveNode also detects model-changing endpoints such as:

- `POST /api/pull`
- `DELETE /api/delete`

Those requests trigger a refresh of the advertised model set.

## Docker Upgrade Flow

In Docker-managed mode, an update does this:

1. Pull latest `ollama/ollama`
2. Stop the current container
3. Remove the old container
4. Create and start a replacement
5. Wait until the new Ollama instance responds
6. Trigger refresh and reconnect

Normal proxy/model operations take a read lock during Docker-sensitive operations. Container replacement takes the write lock.

## Failure Model

If a worker connection fails:

1. Log the error
2. Sleep for 10 seconds
3. Reconnect unless shutdown was requested

If a Docker upgrade fails:

- the error is logged
- the process keeps running

## Observability

HiveNode logs:

- startup and configuration events
- authentication and reconnects
- control messages
- proxy lifecycle
- Docker lifecycle
- failures

If InfluxDB is configured, HiveNode also emits request metrics and error records.

## Design Constraints

- HiveNode does not expose a public HTTP server.
- HiveNode depends on HiveCore for queueing and external API behavior.
- Protocol changes must be verified against HiveCore, especially for control-plane responses.
