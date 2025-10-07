//! Primary entrypoint for the stable `blz` CLI binary.

use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    blz_cli::run().await
}
