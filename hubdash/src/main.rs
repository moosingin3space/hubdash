//! The Hubdash binary.

use clap::Parser;
use tracing_subscriber::{EnvFilter, fmt};

/// The Hubdash server CLI.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Socket address to bind to
    #[arg(short, long, env = "BIND_ADDRESS", default_value = "0.0.0.0:3000")]
    bind_address: std::net::SocketAddr,

    /// Enable verbose logging
    #[arg(short, long, env = "LOG_LEVEL", default_value = "info")]
    log_level: String,
}

#[cfg(not(feature = "tokio"))]
fn main() {
    compile_error!("The 'tokio' feature is required for this binary.");
}

#[cfg(feature = "tokio")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    fmt()
        .with_env_filter(EnvFilter::new(&args.log_level))
        .init();

    let router = hubdash::create_router();
    let listener = tokio::net::TcpListener::bind(args.bind_address).await?;
    Ok(axum::serve(listener, router.into_make_service()).await?)
}
