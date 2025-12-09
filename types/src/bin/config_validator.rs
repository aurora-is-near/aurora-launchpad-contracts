use aurora_launchpad_types::config::LaunchpadConfig;
use near_sdk::serde_json;
use std::fmt::Display;

#[derive(near_sdk::serde::Deserialize)]
#[serde(crate = "near_sdk::serde")]
struct ConfigWithAdmin {
    #[allow(dead_code)]
    admin: Option<String>,
    config: LaunchpadConfig,
}

fn main() {
    match validate_config() {
        Ok(()) => println!("Config is OK!!!"),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}

fn validate_config() -> Result<(), Error> {
    let path_to_config = std::env::args().nth(1).ok_or(Error::InvalidPath)?;
    let config = std::fs::read_to_string(path_to_config).map_err(Error::Read)?;
    let args: ConfigWithAdmin = serde_json::from_str(&config).map_err(Error::Deserialize)?;
    // Pass None for timestamp as this is an off-chain validation tool
    args.config.validate(None).map_err(Error::InvalidConfig)
}

#[derive(Debug)]
enum Error {
    InvalidPath,
    Read(std::io::Error),
    Deserialize(serde_json::Error),
    InvalidConfig(&'static str),
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPath => write!(f, "Missing path to the config file"),
            Self::Read(e) => write!(f, "Error reading the config file: {e}"),
            Self::Deserialize(e) => write!(f, "Error deserializing the config file: {e}"),
            Self::InvalidConfig(e) => write!(f, "Invalid config: {e}"),
        }
    }
}
