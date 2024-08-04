use clap::{Subcommand, Args};

#[derive(Args, Debug, Clone)]
pub struct CmdImageExtractConfigArgs {
    pub image: String,
}

#[derive(Subcommand, Debug)]
pub enum ImageCommands {
    /// Extract config from image
    ExtractConfig(CmdImageExtractConfigArgs),
}

