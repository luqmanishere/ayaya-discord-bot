use std::{
    io::{BufRead, BufReader},
    sync::Arc,
};

use serenity::all::{ActivityData, CacheHttp, Context, EventHandler, FullEvent};

use crate::{Data, setup_cookies};

pub struct StartupHandler;

#[serenity::async_trait]
impl EventHandler for StartupHandler {
    async fn dispatch(&self, context: &Context, event: &FullEvent) {
        match event {
            FullEvent::Ready { data_about_bot, .. } => {
                println!("Ready is called!");
                // TODO: migrate setup function
                tracing::info!("Setup is running after Ready Event");
                let data: Arc<Data> = context.data();
                let commands = data.commands.iter().map(|e| e).collect::<Vec<_>>();
                poise::builtins::register_globally(context.http(), commands)
                    .await
                    .unwrap();

                let bot_user_name = &data_about_bot.user.name;
                let session_id = &data_about_bot.session_id;
                let bot_user_id = data_about_bot.user.id;
                tracing::info!(
                    "Logged in as {} with session id {}.",
                    bot_user_name,
                    session_id
                );

                {
                    let mut user_id_lock = data.user_id.write().await;
                    *user_id_lock = bot_user_id;
                }

                // TODO: handle this error
                setup_cookies(
                    &data.data_manager,
                    &data.ytdlp_config_path,
                    &data.secret_key,
                )
                .await
                .expect("handle this error somehow");

                // test yt-dlp
                #[expect(clippy::zombie_processes)]
                let child = std::process::Command::new("yt-dlp")
                    .arg("-v")
                    // .arg("--extractor-args")
                    // .arg("youtube:player_client=web_creator,mweb")
                    .arg("-O")
                    .arg("title,channel")
                    .arg("https://www.youtube.com/watch?v=1aPOj0ERTEc")
                    .stderr(std::process::Stdio::piped())
                    .spawn()
                    .expect("yt-dlp runs");
                let stderr = child
                    .stderr
                    .ok_or_else(|| std::io::Error::other("Could not capture stdout"))
                    .expect("cant get yt-dlp stdout");

                let reader = BufReader::new(stderr);

                reader
                    .lines()
                    .map_while(Result::ok)
                    .for_each(|line| tracing::info!("yt-dlp setup: {}", line));
                tracing::info!("yt-dlp checks done");
                context.set_activity(Some(ActivityData::watching("Hoshimachi Suichan")));
            }
            FullEvent::CacheReady { guilds, .. } => {
                tracing::info!("Cached guild info is ready for {} guilds.", guilds.len());
            }
            _ => {}
        }
    }
}
