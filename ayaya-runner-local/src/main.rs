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

        // use cookies at the same path  as config if in a container
        if !std::env::var("container").unwrap_or_default().is_empty() {
            let cookies_path = yt_dlp_config_dir.join("cookies.txt");
            let yt_dlp_config = format!("--cookies {}", cookies_path.to_str().unwrap_or(""));
            std::fs::write(yt_dlp_config_dir.join("config"), yt_dlp_config).into_diagnostic()?;
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
        std::fs::create_dir_all(&data_dir)
            .into_diagnostic()
            .wrap_err("Unable to create data dir")?;
    }

    let ayayadc = ayayabot(token, None, yt_dlp_config_dir, secret_key, data_dir).await?;
    ayayadc
        .local_bind(net::SocketAddr::V4(SocketAddrV4::new(
            Ipv4Addr::new(127, 0, 0, 1),
            8000,
        )))
        .await
}

fn file_or_env_var(env_name: &str) -> miette::Result<String> {
    let filename_var = format!("{env_name}_FILE");
    if let Ok(token_file) = env::var(&filename_var) {
        std::fs::read_to_string(&token_file)
            .into_diagnostic()
            .context(miette::miette!(
            "File ENV var for {env_name} is set to {token_file} but the contents cannot be read."
        ))
    } else {
        #[cfg(debug_assertions)]
        dotenvy::dotenv().expect("works");
        env::var(env_name)
            .into_diagnostic()
            .context(miette::miette!(
                "Expected a token for {env_name} or a file from {filename_var} in the environment. Please refer to the README.",
            ))
    }
}
