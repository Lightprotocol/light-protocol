use std::sync::Once;

use env_logger::Env;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

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

        let stdout_env_filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

        let stdout_layer = fmt::Layer::new()
            .with_writer(std::io::stdout)
            .with_ansi(true)
            .with_filter(stdout_env_filter);

        if let Some(file_appender) = file_appender {
            let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
            let file_env_filter = EnvFilter::new("info");
            let file_layer = fmt::Layer::new()
                .with_writer(non_blocking)
                .with_filter(file_env_filter);

            tracing_subscriber::registry()
                .with(stdout_layer)
                .with(file_layer)
                .with(env_filter)
                .init();

            std::mem::forget(_guard);
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
