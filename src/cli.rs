//! Faucet options parser.

use crate::{config, version};
use clap::StructOpt;
use std::path::PathBuf;

#[derive(StructOpt)]
#[structopt(name = "faucet:", version = version::display!(), about = "NeonLabs Faucet Service")]
pub struct Application {
    #[structopt(
        parse(from_os_str),
        short,
        long,
        default_value = &config::DEFAULT_CONFIG,
        help = "Path to the config file"
    )]
    pub config: PathBuf,

    #[structopt(subcommand)]
    pub cmd: Command,
}

#[derive(StructOpt)]
pub enum Command {
    #[structopt(about = "Show manual(s)")]
    Man {
        #[structopt(long, help = "Show HTTP API manual")]
        api: bool,
        #[structopt(long, help = "Show configuration file manual")]
        config: bool,
        #[structopt(long, help = "Show environment variables manual")]
        env: bool,
        #[structopt(long, help = "Dump manual in Markdown format")]
        raw: bool,
    },

    #[structopt(about = "Show config")]
    Config {
        #[structopt(
            parse(from_os_str),
            short,
            long,
            default_value = &config::DEFAULT_CONFIG,
            help = "Path to the config file"
        )]
        file: PathBuf,
    },

    #[structopt(about = "Show environment variables")]
    Env {},

    #[structopt(about = "Start listening for requests")]
    Run {
        #[structopt(
            long,
            default_value = &config::AUTO,
            help = "Number of listening workers"
        )]
        workers: String,
    },
}

/// Constructs instance of Application.
pub fn application() -> Application {
    Application::parse()
}
