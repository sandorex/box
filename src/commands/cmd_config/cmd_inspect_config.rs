use crate::{cli, config};
use std::process::ExitCode;

pub fn inspect_config(cli_args: &cli::cli_config::CmdConfigInspectArgs) -> ExitCode {
    match config::ConfigFile::load_from_file(&cli_args.path) {
        Ok(x) => {
            println!("{:#?}", x);

            ExitCode::SUCCESS
        },
        Err(err) => {
            eprintln!("Failed to load config {}: {}", cli_args.path, err);

            ExitCode::SUCCESS
        }
    }
}
