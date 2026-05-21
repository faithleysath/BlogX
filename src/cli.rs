use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};

use crate::build::{BuildOptions, build_site, clean_site};
use crate::config::Config;
use crate::init::{init_project, sync_default_theme};
use crate::serve::{ServeOverrides, serve_site};

#[derive(Debug, Parser)]
#[command(name = "blogx", version, about = "HTML+CSS-first static site compiler")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    Init(InitArgs),
    Build(BuildArgs),
    Serve(ServeArgs),
    Theme(ThemeArgs),
    Clean(CleanArgs),
}

#[derive(Debug, Args)]
pub struct InitArgs {
    pub name: PathBuf,
}

#[derive(Debug, Args)]
pub struct BuildArgs {
    #[arg(long)]
    pub clean: bool,
    #[arg(long)]
    pub no_cache: bool,
    #[arg(long)]
    pub jobs: Option<usize>,
    #[arg(long)]
    pub profile: bool,
}

#[derive(Debug, Args)]
pub struct ServeArgs {
    #[arg(long)]
    pub host: Option<String>,
    #[arg(long)]
    pub port: Option<u16>,
    #[arg(long)]
    pub jobs: Option<usize>,
}

#[derive(Debug, Args)]
pub struct ThemeArgs {
    #[command(subcommand)]
    pub command: ThemeCommand,
}

#[derive(Debug, Subcommand)]
pub enum ThemeCommand {
    Sync(ThemeSyncArgs),
}

#[derive(Debug, Args)]
pub struct ThemeSyncArgs {
    #[arg(long)]
    pub dest: Option<PathBuf>,
    #[arg(long)]
    pub prune: bool,
}

#[derive(Debug, Args)]
pub struct CleanArgs;

pub fn run(cli: Cli) -> Result<()> {
    match cli.command {
        Command::Init(args) => init_project(&args.name),
        Command::Build(args) => {
            let config = Config::load_from_current_dir()?;
            build_site(
                &config,
                BuildOptions {
                    clean: args.clean,
                    no_cache: args.no_cache,
                    jobs: args.jobs,
                    profile: args.profile,
                },
            )?;
            Ok(())
        }
        Command::Serve(args) => {
            let overrides = ServeOverrides {
                host: args.host,
                port: args.port,
            };
            serve_site(overrides, args.jobs)
        }
        Command::Theme(args) => match args.command {
            ThemeCommand::Sync(sync_args) => {
                let config = Config::load_from_current_dir()?;
                let dest = sync_args.dest.unwrap_or(config.paths.theme);
                let stats = sync_default_theme(&dest, sync_args.prune)?;
                println!(
                    "synced default theme to {} ({} files copied, {} files pruned, {} directories pruned)",
                    dest.display(),
                    stats.files_copied,
                    stats.files_pruned,
                    stats.dirs_pruned
                );
                Ok(())
            }
        },
        Command::Clean(_) => {
            let config = Config::load_from_current_dir()?;
            clean_site(&config)
        }
    }
}
