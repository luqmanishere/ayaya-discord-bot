use std::{
    env,
    net::{self, Ipv4Addr, SocketAddrV4},
};

use ayaya_lib::ayayabot;
use miette::{IntoDiagnostic, WrapErr};

#[tokio::main]
async fn main() -> miette::Result<()> {
    let home = std::path::PathBuf::from(std::env::var("HOME").into_diagnostic()?);
    let yt_dlp_config_dir = home.join(".config/yt-dlp");
    if !yt_dlp_config_dir.exists() {
        std::fs::create_dir_all(&yt_dlp_config_dir).into_diagnostic()?;
    }

    // Configure the client with your Discord bot token in the environment.
    // DISCORD_TOKEN_FILE is searched first, then DISCORD_TOKEN.
    // IF DISCORD_TOKEN_FILE is found, the token is read from the file.
    let token = {
        if let Ok(token_file) = env::var("DISCORD_TOKEN_FILE") {
            std::fs::read_to_string(token_file).into_diagnostic()?
        } else {
            #[cfg(debug_assertions)]
            dotenvy::dotenv().expect("works");
            env::var("DISCORD_TOKEN")
                .into_diagnostic()
                .wrap_err("Expected a token in the environment")?
        }
    };

    let db_str = {
        if let Ok(token_file) = env::var("DATABASE_URL_FILE") {
            std::fs::read_to_string(token_file).into_diagnostic()?
        } else {
            #[cfg(debug_assertions)]
            dotenvy::dotenv().expect("finding .env file");
            env::var("DATABASE_URL")
                .into_diagnostic()
                .wrap_err("Expected a token in the environment")?
        }
    };

    let secret_key = {
        if let Ok(token_file) = env::var("AGE_SECRET_KEY_FILE") {
            std::fs::read_to_string(token_file).into_diagnostic()?
        } else {
            #[cfg(debug_assertions)]
            dotenvy::dotenv().expect("works");
            env::var("AGE_SECRET_KEY")
                .into_diagnostic()
                .context("Expected a token in the environment")?
        }
    };

    let ayayadc = ayayabot(token, db_str, None, yt_dlp_config_dir, secret_key).await?;
    ayayadc
        .local_bind(net::SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::new(127, 0, 0, 1),
            8000,
        )))
        .await
}
