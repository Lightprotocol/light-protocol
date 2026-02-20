use std::sync::Once;

use env_logger::Env;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

static LOG_GUARD: std::sync::OnceLock<tracing_appender::non_blocking::WorkerGuard> =
    std::sync::OnceLock::new();

static INIT: Once = Once::new();

pub fn setup_telemetry() {
    INIT.call_once(|| {
        let file_appender = match RollingFileAppender::builder()
            .rotation(Rotation::HOURLY)
            .filename_prefix("forester")
            .filename_suffix("log")
            .max_log_files(48) // 2 days
            .build("logs")
        {
            Ok(appender) => Some(appender),
            Err(e) => {
                eprintln!(
                    "Warning: Failed to create log file appender: {}. Logging to stdout only.",
                    e
                );
                None
            }
        };

        let env_filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

        let stdout_layer = fmt::layer()
            .with_writer(std::io::stdout)
            .compact()
            .with_ansi(true);

        if let Some(file_appender) = file_appender {
            let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
            let _ = LOG_GUARD.set(guard);

            let file_layer = fmt::layer()
                .json()
                .with_current_span(true)
                .with_span_list(true)
                .flatten_event(true)
                .with_ansi(false)
                .with_writer(non_blocking);

            tracing_subscriber::registry()
                .with(stdout_layer)
                .with(file_layer)
                .with(env_filter)
                .init();
        } else {
            tracing_subscriber::registry()
                .with(stdout_layer)
                .with(env_filter)
                .init();
        }
    });
}

pub fn setup_logger() {
    let env = Env::new().filter_or("RUST_LOG", "info,forester=debug");
    env_logger::Builder::from_env(env).init();
}
