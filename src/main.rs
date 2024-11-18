#[cfg(feature = "normal")]
#[tracing::instrument]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    use anyhow::Context as _;
    use ayaya_discord_bot::ayayabot;

    use std::{
        env,
        net::{Ipv4Addr, SocketAddrV4},
    };

    let home =
        std::path::PathBuf::from(std::env::var("HOME").map_err(shuttle_runtime::CustomError::new)?);
    let yt_dlp_config_dir = home.join(".config/yt-dlp");
    if !yt_dlp_config_dir.exists() {
        std::fs::create_dir_all(&yt_dlp_config_dir)?;
    }

    // Configure the client with your Discord bot token in the environment.
    // DISCORD_TOKEN_FILE is searched first, then DISCORD_TOKEN.
    // IF DISCORD_TOKEN_FILE is found, the token is read from the file.
    let token = {
        if let Ok(token_file) = env::var("DISCORD_TOKEN_FILE") {
            std::fs::read_to_string(token_file)?
        } else {
            #[cfg(debug_assertions)]
            dotenvy::dotenv().expect("works");
            env::var("DISCORD_TOKEN").context("Expected a token in the environment")?
        }
    };

    let db_str = {
        if let Ok(token_file) = env::var("DATABASE_URL_FILE") {
            std::fs::read_to_string(token_file)?
        } else {
            #[cfg(debug_assertions)]
            dotenvy::dotenv().expect("works");
            env::var("DATABASE_URL").context("Expected a token in the environment")?
        }
    };

    let secret_key = {
        if let Ok(token_file) = env::var("AGE_SECRET_KEY_FILE") {
            std::fs::read_to_string(token_file)?
        } else {
            #[cfg(debug_assertions)]
            dotenvy::dotenv().expect("works");
            env::var("AGE_SECRET_KEY").context("Expected a token in the environment")?
        }
    };

    let ayayadc = ayayabot(token, db_str, None, yt_dlp_config_dir, secret_key).await?;
    ayayadc
        .local_bind(std::net::SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::new(127, 0, 0, 1),
            8000,
        )))
        .await
}

#[cfg(feature = "shuttle")]
use ayaya_discord_bot::service::AyayaDiscordBot;
#[cfg(feature = "shuttle")]
#[shuttle_runtime::main]
async fn shuttle_main(
    #[shuttle_runtime::Secrets] secret_store: shuttle_runtime::SecretStore,
) -> Result<AyayaDiscordBot, shuttle_runtime::Error> {
    use anyhow::Context;
    use ayaya_discord_bot::{ayayabot, LokiOpts};

    let home =
        std::path::PathBuf::from(std::env::var("HOME").map_err(shuttle_runtime::CustomError::new)?);
    let yt_dlp_config_dir = home.join(".config/yt-dlp");
    if !yt_dlp_config_dir.exists() {
        std::fs::create_dir_all(&yt_dlp_config_dir)?;
    }

    // Install external dependency (in the shuttle container only)
    use std::env;
    if std::env::var("SHUTTLE")
        .unwrap_or_default()
        .contains("true")
    {
        // installs the following packages with apt
        if !std::process::Command::new("apt")
            .arg("update")
            .status()
            .expect("failed to run apt")
            .success()
            || !std::process::Command::new("apt")
                .arg("install")
                .arg("-y")
                .arg("pipx") // the apt package that a dependency of my project needs to compile
                .arg("ffmpeg") // the apt package that a dependency of my project needs to compile
                // can add more here
                .status()
                .expect("failed to run apt")
                .success()
        {
            panic!("failed to install dependencies")
        }
        // installs the following packages with pipx
        if !std::process::Command::new("pipx")
            .arg("install")
            .arg("yt-dlp")
            // can add more here
            .status()
            .expect("failed to run pipx")
            .success()
        {
            panic!("failed to install dependencies")
        }

        // prepend pipx path
        if let Some(path) = env::var_os("PATH") {
            let mut paths = env::split_paths(&path).collect::<Vec<_>>();
            let home = std::path::PathBuf::from(
                std::env::var("HOME").map_err(shuttle_runtime::CustomError::new)?,
            );
            paths.push(home.join(".local/bin/"));
            let new_path = env::join_paths(paths).map_err(shuttle_runtime::CustomError::new)?;
            env::set_var("PATH", &new_path);
        }

        // write yt-dlp config file
        {
            // use cookies at the same path  as config
            let cookies_path = yt_dlp_config_dir.join("cookies.txt");
            let yt_dlp_config = format!("--cookies {}", cookies_path.to_str().unwrap_or(""));
            std::fs::write(yt_dlp_config_dir.join("config"), yt_dlp_config)?;
        }
    }

    // Configure the client with your Discord bot token in the environment.
    // DISCORD_TOKEN_FILE is searched first, then DISCORD_TOKEN.
    // IF DISCORD_TOKEN_FILE is found, the token is read from the file.
    let token = secret_store
        .get("DISCORD_TOKEN")
        .context("'DISCORD_TOKEN' was not found")?;

    let db_str = secret_store
        .get("DATABASE_URL")
        .context("'DATABASE_URL' was not found")?;

    let loki = match secret_store.get("GRAFANA_USER") {
        Some(grafana_user) => {
            let grafana_api_key = secret_store
                .get("GRAFANA_API_KEY")
                .context("'GRAFANA_API_KEY' is not found")?;
            let application_log_label = secret_store
                .get("APPLICATION_LOG_LABEL")
                .context("'APPLICATION_LOG_LABEL' is not found")?;

            Some(LokiOpts {
                grafana_user,
                grafana_api_key,
                application_log_label,
            })
        }
        None => {
            println!("Grafana Loki will not be used");
            None
        }
    };

    let secret_key = secret_store
        .get("AGE_SECRET_KEY")
        .context("'AGE_SECRET_KEY' is not found")?;

    let client = ayayabot(token, db_str, loki, yt_dlp_config_dir, secret_key).await?;
    Ok(client.into())
}
