/// Contains everything related to container configuration

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use std::fmt;

#[derive(Debug)]
struct GenericConfigError(String);

impl fmt::Display for GenericConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Error for GenericConfigError {}

/// Whole config file
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
struct ConfigFile {
    /// Version of the configuration
    pub version: Option<u64>,

    /// All container configs
    pub config: Option<Vec<Config>>,
}

impl ConfigFile {
    /// Loads config from str, path is just for error message and can be anything
    pub fn load_from_str(text: &str) -> Result<Self, Box<dyn Error>> {
        let obj = toml::from_str::<ConfigFile>(text)?;

        let version = obj.version.unwrap_or(1);
        if version != 1 {
            return Err(
                Box::new(GenericConfigError(format!("Invalid schema version {}", version)))
            )
        }

        Ok(obj)
    }

    // TODO make these return the error somehow
    pub fn load_from_file(path: &str) -> Result<Self, Box<dyn Error>> {
        let file_contents = std::fs::read_to_string(path)?;
        Self::load_from_str(&file_contents)
    }

    // TODO move this function out of impl
    /// Load config from directory (loads *.toml file)
    pub fn load_from_dir(path: &str) -> Result<HashMap<String, Config>, Box<dyn Error>> {
        // NOTE there is no handling of different versions here yet
        let mut configs: HashMap<String, Config> = HashMap::new();

        let toml_files: Vec<std::path::PathBuf> = std::path::Path::new(path)
            .read_dir()?
            .map(|x| x.unwrap().path() )
            .filter(|x| x.extension().unwrap_or_default() == "toml")
            .collect();

        for file in toml_files {
            let config_file = match Self::load_from_file(file.display().to_string().as_str()) {
                Ok(x) => x,
                Err(err) => {
                    eprintln!("Error while parsing config file {}:", file.display());
                    eprintln!("{}\n", err);
                    continue;
                },
            };

            for config in config_file.config.unwrap_or_default() {
                // ignore any duplicates, let the user handle it if they wish
                if configs.contains_key(&config.name) {
                    eprintln!("Ignoring duplicate config {} in {}", &config.name, file.display());
                    continue;
                }

                configs.insert(config.name.clone(), config);
            }
        }

        Ok(configs)
    }
}

/// Single configuration for a container, contains default settings and optional settings per
/// engine that get applied over the default settings
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
struct Config {
    // TODO figure out rules for local containers that need to be built
    /// Name of the configuration
    pub name: String,

    /// Image used for the container
    pub image: String,

    /// Optional name to set for the container, otherwise randomly generated
    pub container_name: Option<String>,

    /// Dotfiles directory to use as /etc/skel
    pub dotfiles: Option<String>,

    /// Default setting used regardless of the engine
    pub default: Option<EngineConfig>,

    /// Override default settings if the engine is podman
    pub podman: Option<EngineConfig>,

    /// Override default settings if the engine is docker
    pub docker: Option<EngineConfig>,
}

// TODO create conversion between cli args and this, so one could generate it from cmd args
/// Container arguments for specific engine
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
struct EngineConfig {
    // NOTE keep it simple, do not add unecessary wrappers for arguments

    /// Arguments to pass to the engine verbatim
    pub engine_args: Option<Vec<String>>,

    /// Capabilties to add / remove for the container
    ///
    /// For example `!cap_net_broadcast` disables the capability
    pub capabilities: Option<Vec<String>>,

    /// Environment variables to set
    pub env: Option<HashMap<String, String>>
}

// this is a pain in the arse to write by hand
/*
impl std::ops::Add for SingleConfig {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            extends: Some(
                         [
                         rhs.extends.unwrap_or_default().as_slice(),
                         self.extends.unwrap_or_default().as_slice(),
                         ].concat()
                     ),
            container_name: self.container_name.or(rhs.container_name),
            image: self.image.or(rhs.image),
            dotfiles: self.dotfiles.or(rhs.dotfiles),
            engine_args: Some(
                         [
                         rhs.engine_args.unwrap_or_default().as_slice(),
                         self.engine_args.unwrap_or_default().as_slice(),
                         ].concat()
                     ),
            env: Some(
                        self.env.unwrap_or_default().extend(rhs.env.unwrap_or_default().iter())
                     ),
        }
        // Self::default()
    }
    // fn add_assign(&mut self, rhs: Self) {
        // self.extends.map_or_else(
        //     || rhs.extends.clone(),
        //     |mut x| { x.extend(rhs.extends); Some(x) }
        // );

        // self.name = self.name.or_else();
        // if self.name.is_none() {
        //     self.name = rhs.name.clone();
        // }
        // TODO ...
    // }
}
*/

// TODO add fn to merge specific engine to default config
// impl Config {
//     /// Checks if config has all keys needed defined
//     pub fn check_soundness(&self) -> bool {
//         false
//     }
// }

pub fn test() {
    let s = r#"

[[config]]
name = "fuck"
image = "fedora"

[config.podman]
engine_args = ["--env X=1"]

"#;

    // TODO print error nicely
    // let obj = ConfigFile::load_from_str(s, None).unwrap();
    let obj = ConfigFile::load_from_dir(std::env::current_dir().unwrap().display().to_string().as_str()).unwrap();

    println!("got: {:#?}", obj);
}

