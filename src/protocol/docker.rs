use std::{collections::HashMap, env, fmt::format, sync::RwLock, time::Duration};

use bollard::{container::{Config, CreateContainerOptions, ListContainersOptions, RemoveContainerOptions, StartContainerOptions, StopContainerOptions}, image::CreateImageOptions, secret::{HostConfig, PortBinding}, Docker};
use futures::TryStreamExt;
use indicatif::{ProgressBar, ProgressDrawTarget, ProgressStyle};
use log::info;
use once_cell::sync::Lazy;
use reqwest::blocking::Client;
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

pub async fn start_ollama_docker(models_dir: &str) -> anyhow::Result<String> {
    let key = env::var("HIVE_KEY").expect("HIVE_KEY");
    let port = env::var("OLLAMA_PORT").expect("OLLAMA_PORT");


    let container_name = format!("ollama-hive-{}", &key[..5]);
    if let Some(id) = find_running(&container_name).await? {
        info!("Hive Ollama container already running...");
        return Ok(id);
    } 

    let docker = Docker::connect_with_local_defaults()?;

    // 1. ensure the image is present
    let pull_opts = CreateImageOptions {
        from_image: "ollama/ollama",
        tag: "latest",
        ..Default::default()
    };

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

    info!("Checking for latest ollama/ollama docker image..."); //
    info!("Downloading latest ollama/ollama docker image...");
    let mut stream = docker.create_image(Some(pull_opts), None, None);
    while let Some(pull_info) = stream.try_next().await? {
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


    // port bindings
    let mut port_bindings = HashMap::new();
    port_bindings.insert(
        format!("{}/tcp", port),
        Some(vec![PortBinding {
            host_ip: Some("0.0.0.0".to_string()),
            host_port: Some(port.to_string()),
        }]),
    );

    let host_config = HostConfig {
        binds: Some(vec![format!("{}:/root/.ollama", models_dir)]),
        port_bindings: Some(port_bindings),
        auto_remove: Some(true),
        ..Default::default()
    };

    let bind = format!("{}/tcp", port);
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

    let container = docker
        .create_container::<&str, &str>(Some(CreateContainerOptions {
                name: &container_name, 
                platform: Default::default() 
            }),
            create_opts)
        .await?;
    let id = container.id;

    docker.start_container(&id, None::<StartContainerOptions<String>>).await?;

    // wait for health
    let client = reqwest::blocking::Client::new();
    for _ in 0..20 {
        if client
            .get(format!("http://127.0.0.1:{}/api/version", port))
            .send()
            .is_ok()
        {
            break;
        }
        sleep(Duration::from_millis(500)).await;
    }

    Ok(id)
}

async fn stop_ollama_docker(id: &str) -> anyhow::Result<()> {
    let docker = Docker::connect_with_local_defaults()?;
    docker
        .remove_container(
            id,
            Some(RemoveContainerOptions {
                force: true,
                ..Default::default()
            }),
        )
        .await?;
    Ok(())
}

/// upgrade the ollama container in place, re-using the same host models_dir mount
pub async fn upgrade_ollama_docker(models_dir: &str) -> Result<String> {
    let key = env::var("HIVE_KEY").expect("HIVE_KEY");
    let port = env::var("OLLAMA_PORT").expect("OLLAMA_PORT");
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

    let _write_guard = DOCKER_UPGRADE_LOCK.write().unwrap();
    
    // 2 stop & remove the old container if it exists
    let _ = docker.stop_container(&container_name, None::<StopContainerOptions>).await;
    let _ = docker.remove_container(
        &container_name,
        Some(RemoveContainerOptions { force: true, ..Default::default() }),
    ).await?;

    // 3 create & start a fresh container, mounting the same models_dir
    // port bindings
    let mut port_bindings = HashMap::new();
    let bind = format!("{}/tcp", port);
    port_bindings.insert(
        bind.clone(),
        Some(vec![PortBinding {
            host_ip: Some("0.0.0.0".to_string()),
            host_port: Some(port.to_string()),
        }]),
    );

    let host_config = HostConfig {
        binds: Some(vec![format!("{}:/root/.ollama", models_dir)]),
        port_bindings: Some(port_bindings),
        auto_remove: Some(true),
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

    let container = docker
        .create_container::<&str, &str>(Some(CreateContainerOptions {
                name: &container_name, 
                platform: Default::default() 
            }),
            create_opts)
        .await?;
    let id = container.id;


    // 4 wait for Ollama’s HTTP API to come back up
    let client = Client::new();
    for _ in 0..20 {
        if client
            .get(format!("http://127.0.0.1:{}/api/version", port))
            .send()
            .is_ok()
        {
            break;
        }
        sleep(Duration::from_millis(500)).await;
    }

    Ok(id)
}