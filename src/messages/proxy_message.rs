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
        Self { protocol, method, uri, headers, body}
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
    
    pub fn modifies_poll(&self) -> bool {
        match (self.protocol.as_str(), self.method.as_str(), self.uri.as_str()) {
            ("HTTP/1.1", "POST", "/api/pull") => true,
            ("HTTP/1.1", "DELETE", "/api/delete") => true,
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