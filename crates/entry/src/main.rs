use std::path::Path;

use alterbot::{consts, start_bot};
use alterconfig::ConfigManager;
use anyhow::Result;

use opentelemetry::KeyValue;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{Resource, logs::SdkLoggerProvider};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

#[allow(unused)]
macro_rules! info { ($($arg:tt)*) => { tracing::info!(target: "alterbot::main", $($arg)*) }; }
macro_rules! warn { ($($arg:tt)*) => { tracing::warn!(target: "alterbot::main", $($arg)*) }; }
#[allow(unused)]
macro_rules! error { ($($arg:tt)*) => { tracing::error!(target: "alterbot::main", $($arg)*) }; }

fn print_startup_info() {
    println!("==========================================");
    println!("alter-bot version {} by gurkan", consts::VERSION);
    println!("MPL 2.0 license");
    println!("{}", consts::REPOSITORY);
    println!("==========================================");
}

fn load_config_manager() -> Result<ConfigManager> {
    let default_path = Path::new(consts::DATA_DIR).join("alter-bot.toml");

    if !default_path.exists() {
        warn!("{} does not exist", default_path.display());

        if std::env::var("CREATE_CONFIG_FILE_IF_NOT_EXIST").unwrap_or_default() == "1" {
            warn!(
                "creating an empty config file at: {}",
                default_path.display()
            );

            if let Some(parent) = default_path.parent() {
                std::fs::create_dir_all(parent)?;
            }

            std::fs::write(&default_path, "")?;
        }
    }

    ConfigManager::new(Some(&default_path))
}

fn init_otel_logger(config_manager: &ConfigManager) -> Option<SdkLoggerProvider> {
    let telemetry_config = config_manager.get_sync().otlp?;

    if telemetry_config.url.trim().is_empty() {
        return None;
    }

    let exporter = opentelemetry_otlp::LogExporter::builder()
        .with_tonic()
        .with_endpoint(&telemetry_config.url)
        .build()
        .ok()?;

    let resource = Resource::builder()
        .with_attributes([
            KeyValue::new("service.name", "alter-bot"),
            KeyValue::new("service.namespace", "production"),
        ])
        .build();

    let provider = SdkLoggerProvider::builder()
        .with_resource(resource)
        .with_batch_exporter(exporter)
        .build();

    Some(provider)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls ring crypto provider");

    let config_manager = load_config_manager()?;

    let logger_provider = init_otel_logger(&config_manager);

    let otel_log_layer = logger_provider.as_ref().map(|provider| {
        opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge::new(provider)
    });

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("warn,alterbot=debug,twilight_gateway::shard=off,twilight_http_ratelimiting::in_memory=info")
    });

    let timer = fmt::time::ChronoLocal::new("%Y-%m-%d %H:%M:%S".to_string());

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().compact().with_target(true).with_timer(timer))
        .with(otel_log_layer)
        .init();

    print_startup_info();

    let sync_commands = std::env::args().nth(1).as_deref() == Some("sync");
    let exit_after_sync = sync_commands && cfg!(debug_assertions);

    config_manager.watch()?;

    let res = start_bot(config_manager, sync_commands, exit_after_sync).await;

    if let Some(provider) = logger_provider {
        let _ = provider.shutdown();
    }

    res?;

    Ok(())
}
