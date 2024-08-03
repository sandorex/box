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
#[derive(Debug, Clone, PartialEq, Default, Deserialize, Serialize)]
pub struct ConfigFile {
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

    pub fn load_from_file(path: &str) -> Result<Self, Box<dyn Error>> {
        let file_contents = std::fs::read_to_string(path)?;
        Self::load_from_str(&file_contents)
    }
}


/// Single configuration for a container, contains default settings and optional settings per
/// engine that get applied over the default settings
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct Config {
    // TODO figure out rules for local containers that need to be built
    /// Name of the configuration
    pub name: String,

    /// Image used for the container
    pub image: String,

    /// Optional name to set for the container, otherwise randomly generated
    #[serde(default)]
    pub container_name: String,

    /// Dotfiles directory to use as /etc/skel
    #[serde(default)]
    pub dotfiles: String,

    /// Default setting used regardless of the engine
    #[serde(default)]
    pub default: EngineConfig,

    /// Override default settings if the engine is podman
    #[serde(default)]
    pub podman: EngineConfig,

    /// Override default settings if the engine is docker
    #[serde(default)]
    pub docker: EngineConfig,
}

impl Config {
    pub fn get_merged_engine_config(&self, engine: crate::util::EngineKind) -> EngineConfig {
        let engine_config = match engine {
            crate::util::EngineKind::Podman => &self.podman,
            crate::util::EngineKind::Docker => &self.docker,
        };

        self.default.clone() + engine_config.clone()
    }
}

// TODO create conversion between cli args and this, so one could generate it from cmd args
/// Container arguments for specific engine
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct EngineConfig {
    // NOTE keep it simple, do not add unecessary wrappers for arguments

    /// Arguments to pass to the engine verbatim
    #[serde(default)]
    pub engine_args: Vec<String>,

    /// Capabilties to add / remove for the container
    ///
    /// For example `!cap_net_broadcast` disables the capability
    #[serde(default)]
    pub capabilities: Vec<String>,

    /// Environment variables to set
    #[serde(default)]
    pub env: HashMap<String, String>
}

impl std::ops::Add for EngineConfig {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        // offload all the logic to AddAssign
        let mut result = self.clone();
        result += rhs;
        result
    }
}

impl std::ops::AddAssign for EngineConfig {
    fn add_assign(&mut self, rhs: Self) {
        self.engine_args.extend(rhs.engine_args.into_iter());
        self.capabilities.extend(rhs.capabilities.into_iter());
        self.env.extend(rhs.env.into_iter())
    }
}

/// Load and merge configs from directory (loads *.toml file)
pub fn load_from_dir(path: &str) -> Result<HashMap<String, Config>, Box<dyn Error>> {
    // NOTE there is no handling of different versions here yet
    let mut configs: HashMap<String, Config> = HashMap::new();

    let toml_files: Vec<std::path::PathBuf> = std::path::Path::new(path)
        .read_dir()?
        .map(|x| x.unwrap().path() )
        .filter(|x| x.extension().unwrap_or_default() == "toml")
        .collect();

    for file in toml_files {
        let config_file = match ConfigFile::load_from_file(file.display().to_string().as_str()) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abi_change() {
        // this test should inform me if i break props of the configs
        //
        // NOTE if this fails then check everything else as there many places with ..default and it
        // will not warn about unused props
        let _ = EngineConfig {
           engine_args: vec![],
           capabilities: vec![],
           env: HashMap::new(),
        };

        let _ = Config {
            name: Default::default(),
            image: Default::default(),
            container_name: Default::default(),
            dotfiles: Default::default(),

            default: Default::default(),
            podman: Default::default(),
            docker: Default::default(),
        };
    }

    #[test]
    fn add() {
        let a = EngineConfig {
            engine_args: vec!["aa".into()],
            capabilities: vec!["aa".into()],
            env: HashMap::from([
                ("a".into(), "b".into()),
            ]),
        };

        let b = EngineConfig {
            engine_args: vec!["bb".into()],
            capabilities: vec!["bb".into()],
            env: HashMap::from([
                ("c".into(), "d".into()),
            ]),
        };

        assert_eq!(a + b, EngineConfig {
            engine_args: vec!["aa".into(), "bb".into()],
            capabilities: vec!["aa".into(), "bb".into()],
            env: HashMap::from([
                ("a".into(), "b".into()),
                ("c".into(), "d".into()),
            ]),
        });
    }

    #[test]
    fn add_assign() {
        let mut a = EngineConfig {
            engine_args: vec!["aa".into()],
            capabilities: vec!["aa".into()],
            env: HashMap::from([
                ("a".into(), "b".into()),
            ]),
        };

        let b = EngineConfig {
            engine_args: vec!["bb".into()],
            capabilities: vec!["bb".into()],
            env: HashMap::from([
                ("c".into(), "d".into()),
            ]),
        };

        a += b;

        assert_eq!(a, EngineConfig {
            engine_args: vec!["aa".into(), "bb".into()],
            capabilities: vec!["aa".into(), "bb".into()],
            env: HashMap::from([
                ("a".into(), "b".into()),
                ("c".into(), "d".into()),
            ]),
        });
    }

    #[test]
    fn from_str() {
        let cfg_text = r#"
[[config]]
name = "first"
image = "fedora"

[config.default]
engine_args = [ "default" ]

[config.podman]
engine_args = [ "podman" ]

[config.docker]
engine_args = [ "docker" ]
"#;

        let result = ConfigFile::load_from_str(cfg_text);
        assert!(result.is_ok(), "result is err: {}", result.unwrap_err());
        let result_ok = result.unwrap();

        assert_eq!(result_ok, ConfigFile {
            version: None,
            config: Some(vec![
                Config {
                    name: "first".into(),
                    image: "fedora".into(),

                    default: EngineConfig {
                        engine_args: vec!["default".into()],
                        ..Default::default()
                    },

                    podman: EngineConfig {
                        engine_args: vec!["podman".into()],
                        ..Default::default()
                    },

                    docker: EngineConfig {
                        engine_args: vec!["docker".into()],
                        ..Default::default()
                    },

                    ..Default::default()
                },
            ]),
        });
    }

    #[test]
    fn get_merged_engine_config() {
        use crate::util::EngineKind;

        {
            // sanity test basically?
            let a = Config {
                ..Default::default()
            };

            assert_eq!(a.get_merged_engine_config(EngineKind::Podman), a.default);
            assert_eq!(a.get_merged_engine_config(EngineKind::Docker), a.default);
        }

        {
            let a = Config {
                default: EngineConfig {
                    engine_args: vec!["aa".into()],
                    ..Default::default()
                },
                ..Default::default()
            };

            assert_eq!(a.get_merged_engine_config(EngineKind::Podman), a.default);
            assert_eq!(a.get_merged_engine_config(EngineKind::Docker), a.default);
        }

        {
            // TODO Add all the props here
            let a = Config {
                default: EngineConfig {
                    engine_args: vec!["aa".into()],
                    ..Default::default()
                },
                podman: EngineConfig {
                    engine_args: vec!["bb".into()],
                    ..Default::default()
                },
                docker: EngineConfig {
                    engine_args: vec!["cc".into()],
                    ..Default::default()
                },
                ..Default::default()
            };

            assert_eq!(
                a.get_merged_engine_config(EngineKind::Podman),
                EngineConfig {
                    engine_args: vec!["aa".into(), "bb".into()],
                    ..Default::default()
                }
            );

            assert_eq!(
                a.get_merged_engine_config(EngineKind::Docker),
                EngineConfig {
                    engine_args: vec!["aa".into(), "cc".into()],
                    ..Default::default()
                }
            );
        }
    }
}

pub fn test() {
    let s = r#"

[[config]]
name = "fuck"
image = "fedora"

[config.podman]
engine_args = ["--env X=1"]

"#;

    // TODO print error nicely
    let obj = ConfigFile::load_from_str(s).unwrap();
    // let obj = load_from_dir(std::env::current_dir().unwrap().display().to_string().as_str()).unwrap();

    println!("got: {:#?}", obj);
}

