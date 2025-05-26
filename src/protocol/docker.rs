use bollard::errors::Error as BollardError; // Add this to your use statements
use bollard::container::{Config, CreateContainerOptions, ListContainersOptions, RemoveContainerOptions, StartContainerOptions, StopContainerOptions};
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
use anyhow::Result;

pub static DOCKER_UPGRADE_LOCK: Lazy<RwLock<()>> = Lazy::new(|| RwLock::new(()));

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
    Ok(list.into_iter().next().map(|c| c.id.unwrap()))
}

pub async fn start_ollama_docker() -> anyhow::Result<String> {
    let models_dir = env::var("HIVE_OLLAMA_MODELS").expect("HIVE_OLLAMA_MODELS must be set");
    let key = env::var("HIVE_KEY").expect("HIVE_KEY");
    let port = env::var("OLLAMA_PORT").expect("OLLAMA_PORT");

    let container_name = format!("ollama-hive-{}", &key[..5]);

    // 1. Check if it's already RUNNING. If so, we're done.
    if let Some(id) = find_running(&container_name).await? {
        info!("Hive Ollama container already running (ID: {}).", id);
        return Ok(id);
    }

    // If not running, connect to Docker.
    let docker = Docker::connect_with_local_defaults()?;

    // 2. Try to REMOVE any container (likely stopped) with the same name.
    // This cleans up before we try to create.
    info!("Ensuring no stopped container with name {} exists...", container_name);
    match docker.remove_container(
        &container_name,
        Some(RemoveContainerOptions { force: true, ..Default::default() }), // Use force to remove even if in a weird state (but not running)
    ).await {
        Ok(_) => info!("Removed existing stopped container {}.", container_name),
        Err(BollardError::DockerResponseServerError { status_code: 404, .. }) => {
             info!("No existing container {} found. Good to proceed.", container_name);
        }
        Err(e) => {
            // If another error occurs, something is wrong, so we should fail.
            error!("Error trying to remove existing container {}: {}", container_name, e);
            return Err(e.into());
        }
    }

    // 3. ensure the image is present (ProgressBar logic as before)
    info!("Checking for latest ollama/ollama docker image...");
    let pull_opts = CreateImageOptions {
        from_image: "ollama/ollama:0.6.8",
        tag: "latest",
        ..Default::default()
    };
    let pb_pull = ProgressBar::new(0); // ... (Set style as before) ...
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
    while let Some(pull_info) = stream.try_next().await? { // ... (Handle progress as before) ...
        if let Some(status) = pull_info.status { pb_pull.set_message(status); }
        if let Some(pd) = &pull_info.progress_detail {
            if let (Some(cur), Some(total)) = (pd.current, pd.total) {
                if total > 0 { pb_pull.set_length(total as u64); pb_pull.set_position(cur as u64); }
                else { pb_pull.tick(); }
            } else { pb_pull.tick(); }
        } else { pb_pull.tick(); }
    }
    pb_pull.finish_with_message("Image pulled");


    // 4. Set up config - ENSURE auto_remove IS GONE
    let mut port_bindings = HashMap::new();
    port_bindings.insert(
        format!("11434/tcp"),
        Some(vec![PortBinding {
            host_ip: Some("0.0.0.0".to_string()),
            host_port: Some(port.to_string()),
        }]),
    );

    // Get GPU requests based on environment
    let device_requests = get_gpu_device_requests(); // <-- USE THE HELPER

    let host_config = HostConfig {
        binds: Some(vec![format!("{}:/root/.ollama", models_dir)]),
        port_bindings: Some(port_bindings),
        device_requests,
        ..Default::default()
    };

    // let host_config = HostConfig {
    //     binds: Some(vec![format!("{}:/root/.ollama", models_dir)]),
    //     port_bindings: Some(port_bindings),
    //     // auto_remove: Some(true), // <-- MAKE SURE THIS IS STILL REMOVED
    //     ..Default::default()
    // };

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

    // 5. Create the container (should succeed now)
    info!("Creating container {}...", container_name);
    let container = docker
        .create_container::<&str, &str>(Some(CreateContainerOptions {
                name: &container_name,
                platform: Default::default()
            }),
            create_opts)
        .await?;
    let id = container.id.clone(); // Clone before moving

    // 6. Start the container
    info!("Starting container {} ({})...", container_name, id);
    docker.start_container(&id, None::<StartContainerOptions<String>>).await?;

    // 7. wait for health (as before)
    info!("Waiting for container to become healthy...");
    let client = reqwest::blocking::Client::new();
    for i in 0..60 {
        if client
            .get(format!("http://127.0.0.1:{}/api/version", port))
            .send()
            .is_ok()
        {
            info!("Container is healthy!");
            return Ok(id); // Return success
        }
        info!("Waiting... ({}/60)", i+1);
        sleep(Duration::from_millis(1000)).await;
    }

    // If loop finishes without health, it's an error
    Err(anyhow::anyhow!("Container did not become healthy in time."))
}

// Your stop_ollama_docker and upgrade_ollama_docker functions remain as they were.
// Make sure upgrade_ollama_docker ALSO does NOT use auto_remove.
pub async fn stop_ollama_docker(id: &str) -> anyhow::Result<()> {
    // ... (as before) ...
    let docker = Docker::connect_with_local_defaults()?;
    info!("Stopping container {}...", id);
    match docker.stop_container(id, None::<StopContainerOptions>).await {
         Ok(_) => info!("Stopped container {}", id),
         Err(BollardError::DockerResponseServerError { status_code: 304, .. }) => {
             info!("Container {} was already stopped.", id);
         }
         Err(BollardError::DockerResponseServerError { status_code: 404, .. }) => {
            info!("Container {} not found for stopping.", id);
        }
        Err(e) => warn!("Error stopping container {}: {}", id, e),
    }

    info!("Removing container {}...", id);
     match docker.remove_container(
            id,
            Some(RemoveContainerOptions {
                force: true, // Force helps if stop failed or didn't complete
                ..Default::default()
            }),
        )
        .await {
            Ok(_) => info!("Removed container {}", id),
            Err(BollardError::DockerResponseServerError { status_code: 404, .. }) => {
                info!("Container {} not found for removal.", id);
            }
            Err(e) => {
                 error!("Failed to remove container {}: {}", id, e);
                 return Err(e.into());
            }
        }
    Ok(())
}

pub async fn upgrade_ollama_docker() -> Result<String> {
    // ... (as before, ensuring auto_remove is NOT used) ...
    let models_dir = env::var("HIVE_OLLAMA_MODELS").expect("HIVE_OLLAMA_MODELS must be set");
    let key = env::var("HIVE_KEY").expect("HIVE_KEY");
    let port = env::var("OLLAMA_PORT").expect("OLLAMA_PORT");
    let ollama_url = env::var("OLLAMA_URL").expect("OLLAMA_URL");
    let container_name = format!("ollama-hive-{}", &key[..5]);


    let docker = Docker::connect_with_local_defaults()?;

    // 1 pull the newest image with a progress bar
    let mut pull_stream = docker.create_image(
        Some(CreateImageOptions {
            from_image: "ollama/ollama",
            tag: "latest",
            ..Default::default()
        }),
        None,
        None,
    );

    let pb_pull = ProgressBar::new(0); // ProgressBar for image pulling
    pb_pull.set_draw_target(ProgressDrawTarget::stdout());
    // A good "sweet spot" style: spinner, elapsed time, bar, byte count, ETA, and message
    pb_pull.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] \
             {bytes}/{total_bytes} ({eta}) {wide_msg}",
        )
        .unwrap()
        .progress_chars("█▇▆▅▄▃▂   ")
        .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ "), // Common spinner characters
    );
    pb_pull.set_message("Pulling ollama/ollama:latest...");

    while let Some(pull_info) = pull_stream.try_next().await? {
        if let Some(status) = pull_info.status {
            pb_pull.set_message(status);
        }

        if let Some(pd) = &pull_info.progress_detail {
            if let (Some(cur), Some(total)) = (pd.current, pd.total) {
                if total > 0 { // Ensure total is valid before setting
                    pb_pull.set_length(total as u64);
                    pb_pull.set_position(cur as u64);
                } else {
                    pb_pull.tick(); // If no valid total/current, at least tick the spinner
                }
            } else {
                pb_pull.tick(); // No current/total available
            }
        } else {
            pb_pull.tick(); // No progress_detail, just a status update, so tick spinner
        }
    }
    pb_pull.finish_with_message("Image pulled");

    info!("Stopping Docker container");

    warn!("Waiting to gain control of the docker connection from other threads");
    let _write_guard = DOCKER_UPGRADE_LOCK.write().unwrap();
    warn!("Got control!");

    // 2 stop & remove the old container if it exists
    match docker.stop_container(&container_name, None::<StopContainerOptions>).await {
        Ok(_) => info!("Stopped container {}", container_name),
        Err(bollard::errors::Error::DockerResponseServerError { status_code: 404, .. }) => {
            info!("Container {} already stopped or not found.", container_name);
        }
        Err(e) => warn!("Error stopping container {}: {}", container_name, e), // Or return Err(e.into()) if stop MUST succeed
    }

    match docker.remove_container(
        &container_name,
        Some(RemoveContainerOptions { force: true, ..Default::default() }),
    ).await {
        Ok(_) => info!("Removed container {}", container_name),
        Err(bollard::errors::Error::DockerResponseServerError { status_code: 404, .. }) => {
             info!("Container {} already removed or not found.", container_name);
        }
        Err(bollard::errors::Error::DockerResponseServerError { status_code: 409, message }) => {
             warn!("Container {} removal conflict (409): {}. Will proceed.", container_name, message);
        }
        Err(e) => {
            error!("Failed to remove container {}: {}", container_name, e);
            return Err(e.into()); // Propagate other errors
        }
    }

    // 3 create & start a fresh container, mounting the same models_dir
    // port bindings
    let mut port_bindings = HashMap::new();
    let bind = format!("11434/tcp");
    port_bindings.insert(
        bind.clone(),
        Some(vec![PortBinding {
            host_ip: Some("0.0.0.0".to_string()),
            host_port: Some(port.to_string()),
        }]),
    );

    // Get GPU requests based on environment
    let device_requests = get_gpu_device_requests(); // <-- USE THE HELPER

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
        .create_container::<&str, &str>(Some(CreateContainerOptions {
                name: &container_name,
                platform: Default::default()
            }),
            create_opts)
        .await?;
    let id = container.id.clone();

    info!("Starting new container {}...", id); // Add log
    docker.start_container(&id, None::<StartContainerOptions<String>>).await?; // Start it


    // 4 wait for Ollama’s HTTP API to come back up
    info!("Waiting for new Ollama container to respond");
    let client = Client::new();
    for _ in 0..20 {
        if client
            .get(format!("{}/api/version", ollama_url))
            .send()
            .is_ok()
        {
            break;
        }
        sleep(Duration::from_millis(500)).await;
    }
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

            // Check if NVML can initialize and if GPUs exist
            let nvml = match Nvml::init() {
                Ok(n) => n,
                Err(e) => {
                    warn!("NVML init failed ({}). Cannot enable GPU support, running in CPU mode.", e);
                    return None;
                }
            };
            match nvml.device_count() {
                Ok(0) | Err(_) => {
                    warn!("No NVIDIA GPUs found or count failed. Cannot enable GPU support, running in CPU mode.");
                    return None;
                }
                Ok(_) => {} // GPUs found, proceed.
            }

            // Handle GPU settings
            if trimmed_setting == "-1" {
                info!("GPU_PASSTHROUGH=-1. Requesting all available GPUs.");
                Some(vec![DeviceRequest {
                    count: Some(-1), // Request all GPUs
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
                    warn!("GPU_PASSTHROUGH='{}' provided no valid IDs. Running in CPU mode.", gpu_setting);
                    None
                } else {
                    info!("GPU_PASSTHROUGH='{}'. Requesting GPUs: {:?}", gpu_setting, device_ids);
                    Some(vec![DeviceRequest {
                        device_ids: Some(device_ids), // Request specific GPUs
                        capabilities: Some(vec![vec!["gpu".to_string()]]),
                        ..Default::default()
                    }])
                }
            }
        }
        Err(_) => {
            info!("GPU_PASSTHROUGH not set, running in CPU mode.");
            None // Env var not set, run in CPU mode
        }
    }
}