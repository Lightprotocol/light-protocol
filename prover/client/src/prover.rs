use std::{
    fs::{File, OpenOptions},
    path::PathBuf,
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::sleep,
    time::Duration,
};

use tracing::info;

use crate::{
    constants::{HEALTH_CHECK, SERVER_ADDRESS},
    helpers::get_project_root,
};

static IS_LOADING: AtomicBool = AtomicBool::new(false);

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

        if !health_check(10, 1).await && !IS_LOADING.load(Ordering::Relaxed) {
            IS_LOADING.store(true, Ordering::Relaxed);

            let is_ci = std::env::var("CI").is_ok();
            let output_buffer: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));

            if is_ci {
                use tokio::{io::AsyncReadExt, process::Command as TokioCommand};

                let mut command = TokioCommand::new(prover_path);
                command.arg("start-prover").stdin(Stdio::null());
                command.stdout(Stdio::piped());
                command.stderr(Stdio::piped());

                let mut child = command.spawn().expect("Failed to start prover process");

                let stdout = child.stdout.take();
                let stderr = child.stderr.take();
                let output_buffer_stdout = Arc::clone(&output_buffer);
                let output_buffer_stderr = Arc::clone(&output_buffer);

                if let Some(mut stdout) = stdout {
                    tokio::spawn(async move {
                        let mut buffer = vec![0u8; 1024];
                        loop {
                            match stdout.read(&mut buffer).await {
                                Ok(0) => break,
                                Ok(n) => {
                                    let mut buf = output_buffer_stdout.lock().unwrap();
                                    buf.extend_from_slice(&buffer[..n]);
                                }
                                Err(_) => break,
                            }
                        }
                    });
                }

                if let Some(mut stderr) = stderr {
                    tokio::spawn(async move {
                        let mut buffer = vec![0u8; 1024];
                        loop {
                            match stderr.read(&mut buffer).await {
                                Ok(0) => break,
                                Ok(n) => {
                                    let mut buf = output_buffer_stderr.lock().unwrap();
                                    buf.extend_from_slice(&buffer[..n]);
                                }
                                Err(_) => break,
                            }
                        }
                    });
                }

                std::mem::drop(child);
            } else {
                let log_dir = PathBuf::from("test-ledger");
                std::fs::create_dir_all(&log_dir).ok();
                let log_path = log_dir.join("prover.log");
                let log_file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&log_path)
                    .unwrap_or_else(|_| {
                        File::create(&log_path).expect("Failed to create prover log file")
                    });
                let log_file_stderr = log_file.try_clone().ok();

                let child = Command::new(prover_path)
                    .arg("start-prover")
                    .stdin(Stdio::null())
                    .stdout(Stdio::from(log_file))
                    .stderr(log_file_stderr.map(Stdio::from).unwrap_or(Stdio::null()))
                    .spawn()
                    .expect("Failed to start prover process");

                std::mem::drop(child);
            }

            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            let health_result = health_check(120, 1).await;
            if health_result {
                info!("Prover started successfully");
            } else {
                if is_ci {
                    let output = output_buffer.lock().unwrap();
                    let output_str = String::from_utf8_lossy(&output);
                    eprintln!("=== Prover output (stdout/stderr) ===");
                    eprintln!("{}", output_str);
                    eprintln!("=== End of prover output ===");
                }
                println!(
                    "Failed to start prover, health check failed. {:?}",
                    health_result
                );
                panic!("Failed to start prover, health check failed.");
            }
        }
    } else {
        panic!("Failed to find project root.");
    };
}

pub async fn health_check(retries: usize, timeout: usize) -> bool {
    let client = reqwest::Client::new();
    let mut result = false;
    for _ in 0..retries {
        match client
            .get(format!("{}{}", SERVER_ADDRESS, HEALTH_CHECK))
            .send()
            .await
        {
            Ok(_) => {
                result = true;
                break;
            }
            Err(_) => {
                sleep(Duration::from_secs(timeout as u64));
            }
        }
    }
    result
}
