mod util;
mod cli;
mod commands;
mod config;
mod gui;

use clap::Parser;
use cli::CliCommands;
use cli::cli_config::ConfigCommands;
use cli::cli_image::ImageCommands;
use std::process::ExitCode;
use util::{Engine, ExitResultExt};

pub const ENGINE_ERR_MSG: &str = "Failed to execute engine";

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
// TODO add git commit to full version but in a more reliable way
pub const FULL_VERSION: &str = if cfg!(debug_assertions) {
    concat!(env!("CARGO_PKG_VERSION"), "-debug")
} else {
    env!("CARGO_PKG_VERSION")
};

pub use util::ExitResult;

fn main() -> ExitCode {
    let args = cli::Cli::parse();

    // init does not need engine, just get it from environment if needed
    if let CliCommands::Init = args.cmd {
        if !util::is_in_container() {
            eprintln!("Running init outside a container is dangerous, qutting..");
            return ExitCode::FAILURE;
        }

        return commands::container_init().to_exitcode();
    }

    // find and detect engine
    let engine: Engine = if let Some(chosen) = args.engine {
        // test if engine exists in PATH or as a literal path
        if !(util::executable_exists(&chosen) || std::path::Path::new(&chosen).exists()) {
            eprintln!("Engine '{}' not found in PATH or filesystem", &chosen);
            return ExitCode::FAILURE;
        }

        Engine::detect(&chosen).expect("Failed to detect engine kind")
    } else if let Some(found) = Engine::find_available_engine() {
        found
    } else {
        eprintln!("No compatible container engine found in PATH");
        return ExitCode::FAILURE;
    };

    // prevent running with docker for now
    if let util::EngineKind::Docker = engine.kind {
        eprintln!("Docker is not supported at the moment");
        return ExitCode::FAILURE
    }

    match args.cmd {
        CliCommands::Start(x) => commands::start_container(engine, args.dry_run, x),
        CliCommands::Shell(x) => commands::open_shell(engine, args.dry_run, &x),
        CliCommands::Exec(x) => commands::container_exec(engine, args.dry_run, &x),
        CliCommands::GuiRun(x) => commands::gui_run(args.dry_run, x),
        CliCommands::Exists(x) => commands::container_exists(engine, &x),
        CliCommands::Config(subcmd) => match subcmd {
            ConfigCommands::Extract(x) => commands::extract_config(engine, args.dry_run, &x),
            ConfigCommands::Inspect(x) => commands::inspect_config(&x),
        },
        CliCommands::Image(subcmd) => match subcmd {
            ImageCommands::Build(x) => commands::build_image(&engine, args.dry_run, x),
        },
        // CliCommands::Image(subcmd) => { dbg!(subcmd); ExitCode::SUCCESS },
        CliCommands::List => commands::print_containers(engine, args.dry_run),
        CliCommands::Logs(x) => commands::print_logs(&x),
        CliCommands::Kill(x) => commands::kill_container(engine, args.dry_run, &x),
        CliCommands::Init => unreachable!(), // this is handled before
    }.to_exitcode()
}
