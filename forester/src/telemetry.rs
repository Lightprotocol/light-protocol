use env_logger::Env;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

pub fn setup_telemetry() {
    let file_appender = RollingFileAppender::new(Rotation::HOURLY, "logs", "forester.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,forester=debug"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(
            fmt::Layer::new()
                .with_writer(std::io::stdout)
                .with_ansi(true),
        )
        .with(fmt::Layer::new().with_writer(non_blocking).json())
        .init();

    // Keep _guard in scope to keep the non-blocking writer alive
    std::mem::forget(_guard);
}

pub fn setup_logger() {
    let env = Env::new().filter_or("RUST_LOG", "info,forester=debug");
    env_logger::Builder::from_env(env).init();
}
