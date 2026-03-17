use std::collections::HashMap;

use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Serialize)]
pub struct ProxyMessage {
    pub protocol: String,
    pub method: String,
    pub uri: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl From<String> for ProxyMessage {
    fn from(raw_request: String) -> Self {
        let mut lines = raw_request.lines();
        let request_line = lines.next().unwrap_or("");
        let mut headers = HashMap::new();

        // Parse method and URI
        let mut request_parts = request_line.split_whitespace();
        let method = request_parts.next().unwrap_or("").to_string();
        let uri = request_parts.next().unwrap_or("").to_string();
        let protocol = request_parts.next().unwrap_or("").to_string();

        // Parse headers
        for line in &mut lines {
            if line.is_empty() {
                // End of headers
                break;
            }
            let mut parts = line.splitn(2, ": ");
            let name = parts.next().unwrap_or("");
            let value = parts.next().unwrap_or("");
            headers.insert(name.to_string(), value.to_string());
        }

        // Read the body
        let body = lines.collect::<Vec<&str>>().join("\n");
        Self {
            protocol,
            method,
            uri,
            headers,
            body,
        }
    }
}

impl ProxyMessage {
    pub fn new_http_get(uri: &str) -> Self {
        Self {
            protocol: "HTTP/1.1".into(),
            method: "GET".into(),
            uri: uri.into(),
            headers: HashMap::new(),
            body: "\n".into(),
        }
    }

    pub fn worker_command(&self) -> Option<&str> {
        fn is_supported_worker_command(command: &str) -> bool {
            matches!(command, "REBOOT" | "SHUTDOWN" | "UPDATE" | "UPDATE_OLLAMA")
        }

        match (
            self.protocol.as_str(),
            self.method.as_str(),
            self.uri.as_str(),
        ) {
            ("HIVE", command, _) if is_supported_worker_command(command) => Some(command),
            ("HTTP/1.1", "POST", "/worker/command") => {
                let command = self.body.trim();
                if is_supported_worker_command(command) {
                    Some(command)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn modifies_poll(&self) -> bool {
        match (
            self.protocol.as_str(),
            self.method.as_str(),
            self.uri.as_str(),
        ) {
            ("HTTP/1.1", "POST", "/api/pull") => true,
            ("HTTP/1.1", "DELETE", "/api/delete") => true,
            ("HTTP/1.1", "GET", "/api/tags") => true,
            (_, _, _) => false,
        }
    }

    pub fn extract_model(&self) -> Option<String> {
        // Remove any surrounding whitespace (including newlines).
        let trimmed = self.body.trim();
        if trimmed.is_empty() {
            return None;
        }

        // Attempt to parse the trimmed string as JSON.
        let json_value: Value = serde_json::from_str(trimmed).ok()?;

        // If the JSON is an object and has a "model" key as a string, return it.
        json_value.get("model")?.as_str().map(String::from)
    }
}

#[cfg(test)]
mod tests {
    use super::ProxyMessage;

    #[test]
    fn parses_request_line_headers_and_body() {
        let raw = "POST /api/generate HTTP/1.1\r\nContent-Type: application/json\r\nX-Test: yes\r\n\r\n{\"model\":\"llama3\"}".to_string();
        let message = ProxyMessage::from(raw);

        assert_eq!(message.method, "POST");
        assert_eq!(message.uri, "/api/generate");
        assert_eq!(message.protocol, "HTTP/1.1");
        assert_eq!(
            message.headers.get("Content-Type").map(String::as_str),
            Some("application/json")
        );
        assert_eq!(message.body, "{\"model\":\"llama3\"}");
    }

    #[test]
    fn extracts_model_from_json_body() {
        let message = ProxyMessage {
            protocol: "HTTP/1.1".into(),
            method: "POST".into(),
            uri: "/api/generate".into(),
            headers: Default::default(),
            body: "{\"model\":\"mistral\"}\n".into(),
        };

        assert_eq!(message.extract_model().as_deref(), Some("mistral"));
    }

    #[test]
    fn extracts_worker_command_from_http_request_body() {
        let message = ProxyMessage {
            protocol: "HTTP/1.1".into(),
            method: "POST".into(),
            uri: "/worker/command".into(),
            headers: Default::default(),
            body: " UPDATE \n".into(),
        };

        assert_eq!(message.worker_command(), Some("UPDATE"));
    }

    #[test]
    fn extracts_update_ollama_worker_command_from_hive_method() {
        let message = ProxyMessage {
            protocol: "HIVE".into(),
            method: "UPDATE_OLLAMA".into(),
            uri: "/".into(),
            headers: Default::default(),
            body: String::new(),
        };

        assert_eq!(message.worker_command(), Some("UPDATE_OLLAMA"));
    }

    #[test]
    fn extracts_worker_command_from_hive_message_method() {
        let message = ProxyMessage {
            protocol: "HIVE".into(),
            method: "SHUTDOWN".into(),
            uri: String::new(),
            headers: Default::default(),
            body: String::new(),
        };

        assert_eq!(message.worker_command(), Some("SHUTDOWN"));
    }
}
