use anyhow::Result;
use blogx::cli::{Cli, run};
use clap::Parser;

fn main() -> Result<()> {
    run(Cli::parse())
}
