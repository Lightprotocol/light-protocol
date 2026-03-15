use std::{
    io::{Read, Write},
    net::{TcpStream, ToSocketAddrs},
    process::{Command, Stdio},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use tracing::info;
use tokio::time::sleep;

use crate::{
    constants::{HEALTH_CHECK, SERVER_ADDRESS},
    errors::ProverClientError,
    helpers::get_project_root,
};

static IS_LOADING: AtomicBool = AtomicBool::new(false);

pub(crate) fn build_http_client() -> Result<reqwest::Client, ProverClientError> {
    reqwest::Client::builder()
        .no_proxy()
        .build()
        .map_err(|error| {
            ProverClientError::GenericError(format!("failed to build HTTP client: {error}"))
        })
}

fn health_check_once(timeout: Duration) -> bool {
    if prover_listener_present() {
        return true;
    }

    let endpoint = SERVER_ADDRESS
        .strip_prefix("http://")
        .or_else(|| SERVER_ADDRESS.strip_prefix("https://"))
        .unwrap_or(SERVER_ADDRESS);
    let addr = match endpoint.to_socket_addrs().ok().and_then(|mut addrs| addrs.next()) {
        Some(addr) => addr,
        None => return false,
    };

    let mut stream = match TcpStream::connect_timeout(&addr, timeout) {
        Ok(stream) => stream,
        Err(error) => {
            tracing::debug!(?error, endpoint, "prover health TCP connect failed");
            return health_check_once_with_curl(timeout);
        }
    };

    let _ = stream.set_read_timeout(Some(timeout));
    let _ = stream.set_write_timeout(Some(timeout));

    let host = endpoint.split(':').next().unwrap_or("127.0.0.1");
    let request =
        format!("GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n", HEALTH_CHECK, host);
    if let Err(error) = stream.write_all(request.as_bytes()) {
        tracing::debug!(?error, "failed to write prover health request");
        return health_check_once_with_curl(timeout);
    }

    let mut response = [0u8; 512];
    let bytes_read = match stream.read(&mut response) {
        Ok(bytes_read) => bytes_read,
        Err(error) => {
            tracing::debug!(?error, "failed to read prover health response");
            return health_check_once_with_curl(timeout);
        }
    };

    if bytes_read == 0 {
        return false;
    }

    let response = std::str::from_utf8(&response[..bytes_read]).unwrap_or_default();
    response.contains("200 OK")
        || response.contains("{\"status\":\"ok\"}")
        || health_check_once_with_curl(timeout)
}

fn prover_listener_present() -> bool {
    let endpoint = SERVER_ADDRESS
        .strip_prefix("http://")
        .or_else(|| SERVER_ADDRESS.strip_prefix("https://"))
        .unwrap_or(SERVER_ADDRESS);
    let port = endpoint.rsplit(':').next().unwrap_or("3001");

    match Command::new("lsof")
        .args(["-nP", &format!("-iTCP:{port}"), "-sTCP:LISTEN"])
        .output()
    {
        Ok(output) => output.status.success() && !output.stdout.is_empty(),
        Err(error) => {
            tracing::debug!(?error, "failed to execute lsof prover listener check");
            false
        }
    }
}

fn health_check_once_with_curl(timeout: Duration) -> bool {
    let timeout_secs = timeout.as_secs().max(1).to_string();
    let url = format!("{}{}", SERVER_ADDRESS, HEALTH_CHECK);
    match Command::new("curl")
        .args(["-sS", "-m", timeout_secs.as_str(), url.as_str()])
        .output()
    {
        Ok(output) => {
            output.status.success()
                && String::from_utf8_lossy(&output.stdout).contains("{\"status\":\"ok\"}")
        }
        Err(error) => {
            tracing::debug!(?error, "failed to execute curl prover health check");
            false
        }
    }
}

pub async fn spawn_prover() {
    if let Some(_project_root) = get_project_root() {
        let prover_path: &str = {
            #[cfg(feature = "devenv")]
            {
                &format!("{}/{}", _project_root.trim(), "cli/test_bin/run")
            }
            #[cfg(not(feature = "devenv"))]
            {
                println!("Running in production mode, using prover binary");
                "light"
            }
        };

        if health_check(10, 1).await {
            return;
        }

        if IS_LOADING
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            if health_check(120, 1).await {
                return;
            }
            panic!("Failed to start prover, health check failed.");
        }

        let spawn_result = async {
            let mut command = Command::new(prover_path);
            command.arg("start-prover").stdout(Stdio::piped()).stderr(Stdio::piped());
            let mut child = command.spawn().expect("Failed to start prover process");
            let mut child_exit_status = None;

            for _ in 0..120 {
                if health_check(1, 1).await {
                    info!("Prover started successfully");
                    return;
                }

                if child_exit_status.is_none() {
                    match child.try_wait() {
                        Ok(Some(status)) => {
                            tracing::warn!(
                                ?status,
                                "prover launcher exited before health check succeeded; continuing to poll for detached prover"
                            );
                            child_exit_status = Some(status);
                        }
                        Ok(None) => {}
                        Err(error) => {
                            tracing::error!(?error, "failed to poll prover child process");
                        }
                    }
                }

                sleep(Duration::from_secs(1)).await;
            }

            if let Some(status) = child_exit_status {
                panic!(
                    "Failed to start prover, health check failed after launcher exited with status {status}."
                );
            }

            panic!("Failed to start prover, health check failed.");
        }
        .await;

        IS_LOADING.store(false, Ordering::Release);
        spawn_result
    } else {
        panic!("Failed to find project root.");
    };
}

pub async fn health_check(retries: usize, timeout: usize) -> bool {
    let timeout = Duration::from_secs(timeout as u64);
    let retry_delay = timeout;

    for attempt in 0..retries {
        if health_check_once(timeout) {
            return true;
        }

        if attempt + 1 < retries {
            sleep(retry_delay).await;
        }
    }

    false
}
