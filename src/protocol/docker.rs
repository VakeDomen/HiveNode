use anyhow::{Context, Result};
use bollard::container::{
    Config, CreateContainerOptions, ListContainersOptions, RemoveContainerOptions,
    StartContainerOptions, StopContainerOptions,
};
use bollard::errors::Error as BollardError;
use bollard::image::CreateImageOptions;
use bollard::secret::{DeviceRequest, HostConfig, PortBinding};
use bollard::Docker;
use futures::TryStreamExt;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use log::{error, info, warn};
use nvml_wrapper::Nvml;
use once_cell::sync::Lazy;
use reqwest::blocking::Client;
use std::{collections::HashMap, env, sync::RwLock, time::Duration};
use tokio::time::sleep;

pub static DOCKER_UPGRADE_LOCK: Lazy<RwLock<()>> = Lazy::new(|| RwLock::new(()));

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OllamaMode {
    Docker,
    External,
}

impl OllamaMode {
    fn parse(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "" | "docker" => Ok(Self::Docker),
            "external" | "byo" | "bring-your-own" => Ok(Self::External),
            other => Err(anyhow::anyhow!(
                "Unsupported OLLAMA_MODE `{other}`. Use `docker` or `external`."
            )),
        }
    }
}

pub fn get_ollama_mode() -> Result<OllamaMode> {
    match env::var("OLLAMA_MODE") {
        Ok(value) => OllamaMode::parse(&value),
        Err(_) => Ok(OllamaMode::Docker),
    }
}

pub fn is_docker_managed() -> bool {
    matches!(get_ollama_mode(), Ok(OllamaMode::Docker))
}

pub async fn configure_ollama_runtime() -> Result<()> {
    match get_ollama_mode()? {
        OllamaMode::Docker => {
            start_ollama_docker().await?;
            let ollama_port =
                env::var("OLLAMA_PORT").context("OLLAMA_PORT must be set in docker mode")?;
            env::set_var("OLLAMA_URL", format!("http://127.0.0.1:{ollama_port}"));
            info!("Configured Docker-managed Ollama at http://127.0.0.1:{ollama_port}");
        }
        OllamaMode::External => {
            let ollama_url =
                env::var("OLLAMA_URL").context("OLLAMA_URL must be set in external mode")?;
            info!("Configured external Ollama at {ollama_url}");
        }
    }

    Ok(())
}

pub fn configure_ollama_runtime_blocking() -> Result<()> {
    match get_ollama_mode()? {
        OllamaMode::Docker => {
            // Serialize Docker-backed runtime reconciliation so concurrent worker
            // threads do not all try to recreate the same container at once.
            let _write_guard = DOCKER_UPGRADE_LOCK.write().unwrap();
            let rt = tokio::runtime::Runtime::new()
                .context("Failed to create Tokio runtime for Ollama startup")?;
            rt.block_on(configure_ollama_runtime())
        }
        OllamaMode::External => {
            let ollama_url =
                env::var("OLLAMA_URL").context("OLLAMA_URL must be set in external mode")?;
            info!("Configured external Ollama at {ollama_url}");
            Ok(())
        }
    }
}

async fn find_running(container_name: &str) -> Result<Option<String>> {
    let docker = Docker::connect_with_local_defaults()?;
    let opts = ListContainersOptions::<String> {
        all: false, // only running
        filters: {
            let mut f = HashMap::new();
            f.insert("name".into(), vec![container_name.into()]);
            f
        },
        ..Default::default()
    };
    let list = docker.list_containers(Some(opts)).await?;
    Ok(list.into_iter().next().and_then(|c| c.id))
}

async fn wait_for_ollama_http_ready(
    base_url: &str,
    attempts: usize,
    interval: Duration,
) -> Result<()> {
    let client = Client::new();

    for attempt in 1..=attempts {
        match client.get(format!("{base_url}/api/version")).send() {
            Ok(response) if response.status().is_success() => {
                info!("Ollama API is ready at {base_url}");
                return Ok(());
            }
            Ok(response) => {
                info!(
                    "Waiting for Ollama API at {base_url} ({attempt}/{attempts}): HTTP {}",
                    response.status()
                );
            }
            Err(error) => {
                info!("Waiting for Ollama API at {base_url} ({attempt}/{attempts}): {error}");
            }
        }

        sleep(interval).await;
    }

    Err(anyhow::anyhow!(
        "Ollama API at {base_url} did not become ready in time."
    ))
}

pub async fn start_ollama_docker() -> anyhow::Result<String> {
    let models_dir =
        env::var("HIVE_OLLAMA_MODELS").context("HIVE_OLLAMA_MODELS must be set in docker mode")?;
    let key = env::var("HIVE_KEY").context("HIVE_KEY must be set")?;
    let port = env::var("OLLAMA_PORT").context("OLLAMA_PORT must be set in docker mode")?;
    let ollama_url = format!("http://127.0.0.1:{port}");

    let container_name = format!("ollama-hive-{}", &key[..5]);

    // 1. Check if it's already RUNNING. If the HTTP endpoint is not reachable,
    // recycle the container instead of trusting Docker's running state.
    if let Some(id) = find_running(&container_name).await? {
        info!("Hive Ollama container already running (ID: {}).", id);
        match wait_for_ollama_http_ready(&ollama_url, 5, Duration::from_secs(1)).await {
            Ok(()) => return Ok(id),
            Err(error) => {
                warn!(
                    "Running Ollama container {} is not reachable at {}: {}. Recreating it.",
                    container_name, ollama_url, error
                );
            }
        }
    }

    // If not running, connect to Docker.
    let docker = Docker::connect_with_local_defaults()?;

    // 2. Try to REMOVE any container (likely stopped) with the same name.
    // This cleans up before we try to create.
    info!(
        "Ensuring no stopped container with name {} exists...",
        container_name
    );
    match docker
        .remove_container(
            &container_name,
            Some(RemoveContainerOptions {
                force: true,
                ..Default::default()
            }), // Use force to remove even if in a weird state (but not running)
        )
        .await
    {
        Ok(_) => info!("Removed existing stopped container {}.", container_name),
        Err(BollardError::DockerResponseServerError {
            status_code: 404, ..
        }) => {
            info!(
                "No existing container {} found. Good to proceed.",
                container_name
            );
        }
        Err(e) => {
            // If another error occurs, something is wrong, so we should fail.
            error!(
                "Error trying to remove existing container {}: {}",
                container_name, e
            );
            return Err(e.into());
        }
    }

    // 3. ensure the image is present (ProgressBar logic as before)
    info!("Checking for latest ollama/ollama docker image...");
    let pull_opts = CreateImageOptions {
        from_image: "ollama/ollama",
        tag: "latest",
        ..Default::default()
    };
    let pb_pull = ProgressBar::new(0);
    pb_pull.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] \
             {bytes}/{total_bytes} ({eta}) {wide_msg}",
        )
        .unwrap()
        .progress_chars("█▇▆▅▄▃▂   ")
        .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ "),
    );
    pb_pull.set_message("Pulling ollama/ollama:latest...");
    let mut stream = docker.create_image(Some(pull_opts), None, None);
    while let Some(pull_info) = stream.try_next().await? {
        if let Some(status) = pull_info.status {
            pb_pull.set_message(status);
        }
        if let Some(pd) = &pull_info.progress_detail {
            if let (Some(cur), Some(total)) = (pd.current, pd.total) {
                if total > 0 {
                    pb_pull.set_length(total as u64);
                    pb_pull.set_position(cur as u64);
                } else {
                    pb_pull.tick();
                }
            } else {
                pb_pull.tick();
            }
        } else {
            pb_pull.tick();
        }
    }
    pb_pull.finish_with_message("Image pulled");
    let mut port_bindings = HashMap::new();
    port_bindings.insert(
        format!("11434/tcp"),
        Some(vec![PortBinding {
            host_ip: Some("0.0.0.0".to_string()),
            host_port: Some(port.to_string()),
        }]),
    );

    let device_requests = get_gpu_device_requests();

    let host_config = HostConfig {
        binds: Some(vec![format!("{}:/root/.ollama", models_dir)]),
        port_bindings: Some(port_bindings),
        device_requests,
        ..Default::default()
    };
    let bind = format!("11434/tcp");
    let create_opts = Config {
        image: Some("ollama/ollama:latest"),
        host_config: Some(host_config),
        exposed_ports: Some({
            let mut e = HashMap::new();
            e.insert(bind.as_str(), HashMap::new());
            e
        }),
        ..Default::default()
    };
    info!("Creating container {}...", container_name);
    let container = docker
        .create_container::<&str, &str>(
            Some(CreateContainerOptions {
                name: &container_name,
                platform: Default::default(),
            }),
            create_opts,
        )
        .await?;
    let id = container.id.clone();

    info!("Starting container {} ({})...", container_name, id);
    docker
        .start_container(&id, None::<StartContainerOptions<String>>)
        .await?;

    info!("Waiting for container to become healthy...");
    wait_for_ollama_http_ready(&ollama_url, 60, Duration::from_secs(1)).await?;
    info!("Container is healthy!");
    Ok(id)
}

pub async fn upgrade_ollama_docker() -> Result<String> {
    let models_dir =
        env::var("HIVE_OLLAMA_MODELS").context("HIVE_OLLAMA_MODELS must be set in docker mode")?;
    let key = env::var("HIVE_KEY").context("HIVE_KEY must be set")?;
    let port = env::var("OLLAMA_PORT").context("OLLAMA_PORT must be set in docker mode")?;
    let ollama_url = env::var("OLLAMA_URL").context("OLLAMA_URL must be set before upgrade")?;
    let container_name = format!("ollama-hive-{}", &key[..5]);

    let docker = Docker::connect_with_local_defaults()?;

    let mut pull_stream = docker.create_image(
        Some(CreateImageOptions {
            from_image: "ollama/ollama",
            tag: "latest",
            ..Default::default()
        }),
        None,
        None,
    );

    let pb_pull = ProgressBar::new(0);
    pb_pull.set_draw_target(ProgressDrawTarget::stdout());
    pb_pull.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] \
             {bytes}/{total_bytes} ({eta}) {wide_msg}",
        )
        .unwrap()
        .progress_chars("█▇▆▅▄▃▂   ")
        .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ "),
    );
    pb_pull.set_message("Pulling ollama/ollama:latest...");

    while let Some(pull_info) = pull_stream.try_next().await? {
        if let Some(status) = pull_info.status {
            pb_pull.set_message(status);
        }

        if let Some(pd) = &pull_info.progress_detail {
            if let (Some(cur), Some(total)) = (pd.current, pd.total) {
                if total > 0 {
                    // Ensure total is valid before setting
                    pb_pull.set_length(total as u64);
                    pb_pull.set_position(cur as u64);
                } else {
                    pb_pull.tick();
                }
            } else {
                pb_pull.tick();
            }
        } else {
            pb_pull.tick();
        }
    }
    pb_pull.finish_with_message("Image pulled");

    info!("Stopping Docker container");

    warn!("Waiting to gain control of the docker connection from other threads");
    let _write_guard = DOCKER_UPGRADE_LOCK.write().unwrap();
    warn!("Got control!");

    match docker
        .stop_container(&container_name, None::<StopContainerOptions>)
        .await
    {
        Ok(_) => info!("Stopped container {}", container_name),
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 404, ..
        }) => {
            info!("Container {} already stopped or not found.", container_name);
        }
        Err(e) => warn!("Error stopping container {}: {}", container_name, e),
    }

    match docker
        .remove_container(
            &container_name,
            Some(RemoveContainerOptions {
                force: true,
                ..Default::default()
            }),
        )
        .await
    {
        Ok(_) => info!("Removed container {}", container_name),
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 404, ..
        }) => {
            info!("Container {} already removed or not found.", container_name);
        }
        Err(bollard::errors::Error::DockerResponseServerError {
            status_code: 409,
            message,
        }) => {
            warn!(
                "Container {} removal conflict (409): {}. Will proceed.",
                container_name, message
            );
        }
        Err(e) => {
            error!("Failed to remove container {}: {}", container_name, e);
            return Err(e.into());
        }
    }

    let mut port_bindings = HashMap::new();
    let bind = format!("11434/tcp");
    port_bindings.insert(
        bind.clone(),
        Some(vec![PortBinding {
            host_ip: Some("0.0.0.0".to_string()),
            host_port: Some(port.to_string()),
        }]),
    );

    let device_requests = get_gpu_device_requests();

    let host_config = HostConfig {
        binds: Some(vec![format!("{}:/root/.ollama", models_dir)]),
        port_bindings: Some(port_bindings),
        device_requests,
        ..Default::default()
    };

    let create_opts = Config {
        image: Some("ollama/ollama:latest"),
        host_config: Some(host_config),
        exposed_ports: Some({
            let mut e = HashMap::new();
            e.insert(bind.as_str(), HashMap::new());
            e
        }),
        ..Default::default()
    };

    info!("Creating new Docker container");
    let container = docker
        .create_container::<&str, &str>(
            Some(CreateContainerOptions {
                name: &container_name,
                platform: Default::default(),
            }),
            create_opts,
        )
        .await?;
    let id = container.id.clone();

    info!("Starting new container {}...", id);
    docker
        .start_container(&id, None::<StartContainerOptions<String>>)
        .await?;

    info!("Waiting for new Ollama container to respond");
    wait_for_ollama_http_ready(&ollama_url, 20, Duration::from_millis(500)).await?;
    info!("Done updating Ollama container");
    Ok(id)
}

fn get_gpu_device_requests() -> Option<Vec<DeviceRequest>> {
    match env::var("GPU_PASSTHROUGH") {
        Ok(gpu_setting) => {
            let trimmed_setting = gpu_setting.trim();

            if trimmed_setting.is_empty() {
                info!("GPU_PASSTHROUGH is empty, running in CPU mode.");
                return None;
            }

            let nvml = match Nvml::init() {
                Ok(n) => n,
                Err(e) => {
                    warn!(
                        "NVML init failed ({}). Cannot enable GPU support, running in CPU mode.",
                        e
                    );
                    return None;
                }
            };
            match nvml.device_count() {
                Ok(0) | Err(_) => {
                    warn!("No NVIDIA GPUs found or count failed. Cannot enable GPU support, running in CPU mode.");
                    return None;
                }
                Ok(_) => {}
            }

            if trimmed_setting == "-1" {
                info!("GPU_PASSTHROUGH=-1. Requesting all available GPUs.");
                Some(vec![DeviceRequest {
                    count: Some(-1),
                    capabilities: Some(vec![vec!["gpu".to_string()]]),
                    ..Default::default()
                }])
            } else {
                let device_ids: Vec<String> = trimmed_setting
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();

                if device_ids.is_empty() {
                    warn!(
                        "GPU_PASSTHROUGH='{}' provided no valid IDs. Running in CPU mode.",
                        gpu_setting
                    );
                    None
                } else {
                    info!(
                        "GPU_PASSTHROUGH='{}'. Requesting GPUs: {:?}",
                        gpu_setting, device_ids
                    );
                    Some(vec![DeviceRequest {
                        device_ids: Some(device_ids),
                        capabilities: Some(vec![vec!["gpu".to_string()]]),
                        ..Default::default()
                    }])
                }
            }
        }
        Err(_) => {
            info!("GPU_PASSTHROUGH not set, running in CPU mode.");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{OllamaMode, Result};

    #[test]
    fn parses_ollama_modes() -> Result<()> {
        assert_eq!(OllamaMode::parse("docker")?, OllamaMode::Docker);
        assert_eq!(OllamaMode::parse("external")?, OllamaMode::External);
        assert_eq!(OllamaMode::parse("byo")?, OllamaMode::External);
        Ok(())
    }

    #[test]
    fn rejects_unknown_ollama_mode() {
        assert!(OllamaMode::parse("kubernetes").is_err());
    }
}
