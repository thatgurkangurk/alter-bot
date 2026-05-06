use tracing::info;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

mod consts;

fn print_startup_info() {
    info!("alter-bot version {} by gurkan", consts::VERSION);
    info!("MPL 2.0 license");
    info!("{}", &consts::REPOSITORY);
}

fn main() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("warn,alter_bot=info"));

    let timer = fmt::time::ChronoLocal::new("%Y-%m-%d %H:%M:%S".to_string());

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().compact().with_target(true).with_timer(timer))
        .init();

    print_startup_info();
}
