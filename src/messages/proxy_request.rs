use std::collections::HashMap;

#[derive(Debug)]
pub struct ProxyRequest {
    pub protocol: String, 
    pub method: String, 
    pub uri: String, 
    pub headers: HashMap<String, String>, 
    pub body: String,
}

impl From<String> for ProxyRequest {
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