use env_logger::Env;
use std::sync::Once;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer};

static INIT: Once = Once::new();

pub fn setup_telemetry() {
    INIT.call_once(|| {
        let file_appender = RollingFileAppender::new(Rotation::HOURLY, "logs", "forester.log");
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

        let env_filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info,forester=debug"));

        let file_env_filter = EnvFilter::new("info,forester=debug");
        let stdout_env_filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

        let stdout_layer = fmt::Layer::new()
            .with_writer(std::io::stdout)
            .with_ansi(true)
            .with_filter(stdout_env_filter);

        let file_layer = fmt::Layer::new()
            .with_writer(non_blocking)
            .with_filter(file_env_filter);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(stdout_layer)
            .with(file_layer)
            .init();

        // tracing_subscriber::registry()
        //     .with(env_filter)
        //     .with(
        //         fmt::Layer::new()
        //             .with_writer(std::io::stdout)
        //             .with_ansi(true),
        //     )
        //     .with(fmt::Layer::new().with_writer(non_blocking).json())
        //     .init();

        // Keep _guard in scope to keep the non-blocking writer alive
        std::mem::forget(_guard);
    });
}

pub fn setup_logger() {
    let env = Env::new().filter_or("RUST_LOG", "info,forester=debug");
    env_logger::Builder::from_env(env).init();
}
