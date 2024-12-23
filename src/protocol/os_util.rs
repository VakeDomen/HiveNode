use std::process::Command;

static UNKNOWN_VERSION: &str = "Unknown";

pub fn get_ollama_version() -> String {
    let output = Command::new("ollama")
        .arg("--version")
        .output();

    let output = match output {
        Ok(o) => o,
        Err(_) => return UNKNOWN_VERSION.to_string(),
    };

    // Check if the command was successful
    if output.status.success() {
        // Convert the stdout bytes to a String
        let version = match String::from_utf8(output.stdout) {
            Ok(out) => out,
            Err(_) => return UNKNOWN_VERSION.to_string(),
        };
        match version.trim().split(" ").last() {
            Some(v) => v.to_string(),
            None => UNKNOWN_VERSION.to_string(),
        }
    } else {
        UNKNOWN_VERSION.to_string()
    }
}