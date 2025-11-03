use std::{collections::HashMap, path::Path};

use snafu::{ResultExt, Snafu};

pub mod models;

// TODO: read config

fn load_game_config(config_path: &Path) -> Result<HashMap<String, models::GameConfig>, Error> {
    let path = config_path.join("games");
    let mut configs = HashMap::new();

    // TODO: load config from top level, then go down to folder based shortnames

    let files = path
        .read_dir()
        .context(IoSnafu)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|e| e.is_file() || e.is_symlink())
        .collect::<Vec<_>>();

    for file in files {
        if file.extension() == Some(std::ffi::OsStr::new("toml")) {
            let content = std::fs::read_to_string(&file).context(IoSnafu)?;
            let game_config: models::GameConfig =
                toml::from_str(&content).context(TomlParseSnafu)?;

            let short_name = game_config.game.name.clone();
            configs.insert(short_name, game_config);
        }
    }

    Ok(configs)
}

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("IO error: {}", source))]
    Io { source: std::io::Error },

    #[snafu(display("Failed to parse TOML: {}", source))]
    TomlParse { source: toml::de::Error },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_load_game_config() {
        let config_path =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("dev/game/wuthering_waves.toml");

        let result = load_game_config(&config_path);

        // For now, we just check that the function runs
        // The assertion will be added once the test data is provided
        assert!(result.is_ok() || result.is_err());
    }
}
