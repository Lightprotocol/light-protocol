use std::{
    io::{Read, Write},
    net::{TcpStream, ToSocketAddrs},
    process::{Child, Command},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

use tokio::time::sleep;
use tracing::info;

use crate::{
    constants::{HEALTH_CHECK, SERVER_ADDRESS},
    errors::ProverClientError,
    helpers::get_project_root,
};

static IS_LOADING: AtomicBool = AtomicBool::new(false);
const STARTUP_HEALTH_CHECK_RETRIES: usize = 300;

fn has_http_ok_status(response: &[u8]) -> bool {
    response
        .split(|&byte| byte == b'\n')
        .next()
        .map(|status_line| {
            status_line.starts_with(b"HTTP/")
                && status_line.windows(5).any(|window| window == b" 200 ")
        })
        .unwrap_or(false)
}

pub(crate) fn build_http_client() -> Result<reqwest::Client, ProverClientError> {
    reqwest::Client::builder()
        .no_proxy()
        .build()
        .map_err(|error| {
            ProverClientError::GenericError(format!("failed to build HTTP client: {error}"))
        })
}

fn health_check_once(timeout: Duration) -> bool {
    let endpoint = SERVER_ADDRESS
        .strip_prefix("http://")
        .or_else(|| SERVER_ADDRESS.strip_prefix("https://"))
        .unwrap_or(SERVER_ADDRESS);
    let addr = match endpoint
        .to_socket_addrs()
        .ok()
        .and_then(|mut addrs| addrs.next())
    {
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
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        HEALTH_CHECK, host
    );
    if let Err(error) = stream.write_all(request.as_bytes()) {
        tracing::debug!(?error, "failed to write prover health request");
        return health_check_once_with_curl(timeout);
    }

    let mut response = [0_u8; 512];
    let bytes_read = match stream.read(&mut response) {
        Ok(bytes_read) => bytes_read,
        Err(error) => {
            tracing::debug!(?error, "failed to read prover health response");
            return health_check_once_with_curl(timeout);
        }
    };

    bytes_read > 0
        && (has_http_ok_status(&response[..bytes_read]) || health_check_once_with_curl(timeout))
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

async fn wait_for_prover_health(
    retries: usize,
    timeout: Duration,
    child: &mut Child,
) -> Result<(), String> {
    for attempt in 0..retries {
        if health_check_once(timeout) {
            return Ok(());
        }

        match child.try_wait() {
            Ok(Some(status)) => {
                return Err(format!(
                    "prover process exited before health check succeeded with status {status}"
                ));
            }
            Ok(None) => {}
            Err(error) => {
                return Err(format!("failed to poll prover process status: {error}"));
            }
        }

        if attempt + 1 < retries {
            sleep(timeout).await;
        }
    }

    Err(format!(
        "prover health check failed after {} attempts",
        retries
    ))
}

fn monitor_prover_child(mut child: Child) {
    std::thread::spawn(move || match child.wait() {
        Ok(status) => tracing::debug!(?status, "prover launcher exited"),
        Err(error) => tracing::warn!(?error, "failed to wait on prover launcher"),
    });
}

pub async fn spawn_prover() {
    if let Some(_project_root) = get_project_root() {
        let prover_path = {
            #[cfg(feature = "devenv")]
            {
                format!("{}/{}", _project_root.trim(), "cli/test_bin/run")
            }
            #[cfg(not(feature = "devenv"))]
            {
                println!("Running in production mode, using prover binary");
                "light".to_string()
            }
        };

        if health_check(10, 1).await {
            return;
        }

        if IS_LOADING
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            if health_check(STARTUP_HEALTH_CHECK_RETRIES, 1).await {
                return;
            }
            panic!("Failed to start prover, health check failed.");
        }

        let spawn_result = async {
            let mut child = Command::new(&prover_path)
                .arg("start-prover")
                .spawn()
                .unwrap_or_else(|error| panic!("Failed to start prover process: {error}"));

            match wait_for_prover_health(
                STARTUP_HEALTH_CHECK_RETRIES,
                Duration::from_secs(1),
                &mut child,
            )
            .await
            {
                Ok(()) => {
                    monitor_prover_child(child);
                    info!("Prover started successfully");
                }
                Err(error) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    panic!("Failed to start prover: {error}");
                }
            }
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
