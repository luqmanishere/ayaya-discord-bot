use std::{
    env,
    net::{self, Ipv4Addr, SocketAddrV4},
    path::PathBuf,
};

use ayaya_lib::{ayayabot, error::BotError, service::StartupError};
use snafu::{ResultExt, Snafu};

#[tokio::main]
#[snafu::report]
async fn main() -> Result<(), Error> {
    let home = std::path::PathBuf::from(std::env::var("HOME").context(FindHomeSnafu)?);
    let yt_dlp_config_dir = home.join(".config/yt-dlp");
    if !yt_dlp_config_dir.exists() {
        std::fs::create_dir_all(&yt_dlp_config_dir).context(CreateYtConfigSnafu {
            config_path: &yt_dlp_config_dir,
        })?;

        // use cookies at the same path  as config if in a container
        if !std::env::var("container").unwrap_or_default().is_empty() {
            let cookies_path = yt_dlp_config_dir.join("cookies.txt");
            let yt_dlp_config = format!("--cookies {}", cookies_path.to_str().unwrap_or(""));
            let config_path = yt_dlp_config_dir.join("config");
            std::fs::write(&config_path, yt_dlp_config)
                .context(CreateYtConfigSnafu { config_path })?;
        }
    }

    // Configure the client with your Discord bot token in the environment.
    // DISCORD_TOKEN_FILE is searched first, then DISCORD_TOKEN.
    // IF DISCORD_TOKEN_FILE is found, the token is read from the file.
    let token = file_or_env_var("DISCORD_TOKEN")?.trim().to_string();

    let secret_key = file_or_env_var("AGE_SECRET_KEY")?.trim().to_string();

    // data store dir
    // TODO: allow configuration
    let data_dir = home.join(".local/share/ayayadc");
    if !data_dir.exists() {
        std::fs::create_dir_all(&data_dir).context(CreateDataDirSnafu {
            data_dir: &data_dir,
        })?;
    }

    let ayayadc = ayayabot(token, None, yt_dlp_config_dir, secret_key, data_dir)
        .await
        .context(BotSnafu)?;
    ayayadc
        .local_bind(net::SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::new(127, 0, 0, 1),
            8000,
        )))
        .await
        .context(StartupSnafu)?;
    Ok(())
}

#[expect(clippy::result_large_err)]
fn file_or_env_var(env_name: &str) -> Result<String, Error> {
    let filename_var = format!("{env_name}_FILE");
    if let Ok(token_file) = env::var(&filename_var) {
        std::fs::read_to_string(&token_file).context(EnvFileReadSnafu {
            token_name: env_name,
            env_name: filename_var,
            path: token_file,
        })
    } else {
        #[cfg(debug_assertions)]
        dotenvy::dotenv().expect("works");
        env::var(env_name).context(EnvReadSnafu {
            env_name,
            filename_var,
        })
    }
}

#[derive(Snafu, Debug)]
#[expect(clippy::enum_variant_names)]
enum Error {
    #[snafu(display("Error occured during bot initialization: {source}"))]
    BotError { source: BotError },

    #[snafu(display("Error occured during bot startup: {source}"))]
    StartupError { source: StartupError },

    #[snafu(display("Error finding home dir from the environment"))]
    FindHome { source: std::env::VarError },

    #[snafu(display("Error creating data dir at {} : {source}", data_dir.display()))]
    CreateDataDir {
        source: std::io::Error,
        data_dir: PathBuf,
    },

    #[snafu(display("Error creating yt-dlp config at path {} : {source}", config_path.display()))]
    CreateYtConfig {
        source: std::io::Error,
        config_path: PathBuf,
    },

    #[snafu(display("ENV var {env_name} for {token_name} is set to {} but the contents cannot be read: {source}", path.display()))]
    EnvFileRead {
        source: std::io::Error,
        token_name: String,
        env_name: String,
        path: PathBuf,
    },

    #[snafu(display("Expected a token for {env_name} or a file from {filename_var} in the environment. Please refer to the README | Error: {source}"))]
    EnvRead {
        source: std::env::VarError,
        env_name: String,
        filename_var: String,
    },
}
