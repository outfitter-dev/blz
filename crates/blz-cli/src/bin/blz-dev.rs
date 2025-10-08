//! Developer-profile entrypoint for the `blz-dev` CLI binary.

use anyhow::Result;
use blz_core::profile::{self, AppProfile};

#[tokio::main]
async fn main() -> Result<()> {
    profile::set(AppProfile::Dev);
    blz_cli::run().await
}
