use anyhow::{Context, Result};
use log::info;
use reqwest::blocking::{Client, Response};
use reqwest::header::{HeaderName, HeaderValue, AUTHORIZATION};
use serde::Deserialize;
use std::{env, time::Duration};

use crate::messages::proxy_message::ProxyMessage;
use crate::models::tags::{Tags, Version};

use super::docker::{configure_ollama_runtime, configure_ollama_runtime_blocking};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InferenceBackend {
    Ollama,
    Vllm,
}

impl InferenceBackend {
    fn parse(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "" | "ollama" => Ok(Self::Ollama),
            "vllm" => Ok(Self::Vllm),
            other => Err(anyhow::anyhow!(
                "Unsupported INFERENCE_BACKEND `{other}`. Use `ollama` or `vllm`."
            )),
        }
    }

    pub fn poll_command(self) -> &'static str {
        match self {
            Self::Ollama => "POLL-OLLAMA",
            Self::Vllm => "POLL-VLLM",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Ollama => "ollama",
            Self::Vllm => "vllm",
        }
    }
}

pub fn get_backend() -> Result<InferenceBackend> {
    match env::var("INFERENCE_BACKEND") {
        Ok(value) => InferenceBackend::parse(&value),
        Err(_) => Ok(InferenceBackend::Ollama),
    }
}

pub async fn configure_backend_runtime() -> Result<()> {
    match get_backend()? {
        InferenceBackend::Ollama => configure_ollama_runtime().await,
        InferenceBackend::Vllm => {
            let backend_url = backend_base_url()?;
            env::set_var("BACKEND_URL", &backend_url);
            info!("Configured external vLLM backend at {backend_url}");
            Ok(())
        }
    }
}

pub fn configure_backend_runtime_blocking() -> Result<()> {
    match get_backend()? {
        InferenceBackend::Ollama => configure_ollama_runtime_blocking(),
        InferenceBackend::Vllm => {
            let backend_url = backend_base_url()?;
            env::set_var("BACKEND_URL", &backend_url);
            info!("Configured external vLLM backend at {backend_url}");
            Ok(())
        }
    }
}

pub fn discover_models(client: &Client) -> Result<Vec<String>> {
    match get_backend()? {
        InferenceBackend::Ollama => discover_ollama_models(client),
        InferenceBackend::Vllm => discover_vllm_models(client),
    }
}

pub fn backend_version(client: &Client) -> String {
    match get_backend() {
        Ok(InferenceBackend::Ollama) => ollama_version(client),
        Ok(InferenceBackend::Vllm) => "vllm".to_string(),
        Err(_) => "Unknown".to_string(),
    }
}

pub fn make_backend_request(request: &ProxyMessage, client: &Client) -> Result<Response> {
    if request.protocol.eq("HIVE") {
        return Err(anyhow::anyhow!("Can't make HIVE requests to backend."));
    }

    let backend = get_backend()?;
    let backend_url = backend_base_url()?;
    let request_target = format!("{backend_url}{}", request.uri);
    let mut request_builder = client.request(request.method.parse()?, request_target);

    for (key, value) in request.headers.iter() {
        let key_lower = key.to_ascii_lowercase();
        if key_lower != "host" && key_lower != "content-length" {
            if let (Ok(header_name), Ok(header_value)) = (
                HeaderName::from_bytes(key.as_bytes()),
                HeaderValue::from_str(value),
            ) {
                request_builder = request_builder.header(header_name, header_value);
            }
        }
    }

    if backend == InferenceBackend::Vllm
        && !request
            .headers
            .keys()
            .any(|key| key.eq_ignore_ascii_case("authorization"))
    {
        if let Some(api_key) = backend_api_key() {
            request_builder = request_builder.header(AUTHORIZATION, format!("Bearer {api_key}"));
        }
    }

    if !request.body.is_empty() {
        request_builder = request_builder.body(request.body.to_string());
    }

    Ok(request_builder
        .timeout(Duration::from_secs(60 * 30))
        .send()?)
}

fn discover_ollama_models(client: &Client) -> Result<Vec<String>> {
    let req = ProxyMessage::new_http_get("/api/tags");
    let resp = make_backend_request(&req, client)?;
    Ok(Tags::try_from(resp)?
        .models
        .into_iter()
        .flat_map(|model| {
            let name = model.name;
            if name.contains(":latest") {
                vec![name.clone().replace(":latest", ""), name]
            } else {
                vec![name]
            }
        })
        .collect())
}

fn discover_vllm_models(client: &Client) -> Result<Vec<String>> {
    let req = ProxyMessage::new_http_get("/v1/models");
    let resp = make_backend_request(&req, client)?;
    let models_response: VllmModels = serde_json::from_str(&resp.text()?)?;
    Ok(models_response
        .data
        .into_iter()
        .map(|model| model.id)
        .collect())
}

fn ollama_version(client: &Client) -> String {
    let req = ProxyMessage::new_http_get("/api/version");
    let resp = match make_backend_request(&req, client) {
        Ok(resp) => resp,
        Err(_) => return "Unknown".to_string(),
    };
    match Version::try_from(resp) {
        Ok(v) => v.version,
        Err(_) => "Unknown".to_string(),
    }
}

fn backend_base_url() -> Result<String> {
    match get_backend()? {
        InferenceBackend::Ollama => env::var("BACKEND_URL")
            .or_else(|_| env::var("OLLAMA_URL"))
            .context("OLLAMA_URL or BACKEND_URL must be set for Ollama"),
        InferenceBackend::Vllm => env::var("BACKEND_URL")
            .or_else(|_| env::var("VLLM_URL"))
            .context("BACKEND_URL or VLLM_URL must be set for vLLM")
            .map(|url| strip_openai_version_path(&url)),
    }
    .map(|url| url.trim_end_matches('/').to_string())
}

fn strip_openai_version_path(url: &str) -> String {
    url.trim_end_matches('/')
        .trim_end_matches("/v1")
        .to_string()
}

fn backend_api_key() -> Option<String> {
    env::var("BACKEND_API_KEY")
        .or_else(|_| env::var("VLLM_API_KEY"))
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[derive(Debug, Deserialize)]
struct VllmModels {
    data: Vec<VllmModel>,
}

#[derive(Debug, Deserialize)]
struct VllmModel {
    id: String,
}

#[cfg(test)]
mod tests {
    use super::InferenceBackend;

    #[test]
    fn parses_inference_backends() {
        assert_eq!(
            InferenceBackend::parse("ollama").unwrap(),
            InferenceBackend::Ollama
        );
        assert_eq!(
            InferenceBackend::parse("vllm").unwrap(),
            InferenceBackend::Vllm
        );
    }

    #[test]
    fn chooses_poll_command_for_backend() {
        assert_eq!(InferenceBackend::Ollama.poll_command(), "POLL-OLLAMA");
        assert_eq!(InferenceBackend::Vllm.poll_command(), "POLL-VLLM");
    }

    #[test]
    fn strips_openai_version_path_from_vllm_url() {
        assert_eq!(
            super::strip_openai_version_path("http://localhost:8000/v1/"),
            "http://localhost:8000"
        );
    }
}
