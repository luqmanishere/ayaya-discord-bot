use std::sync::Arc;

use poise::serenity_prelude as serenity;

use crate::{error::BotError, Data};

pub struct AyayaDiscordBot {
    pub discord: Discord,
    pub router: axum::Router,
}

pub struct Discord {
    pub framework: poise::Framework<Data, BotError>,
    pub token: String,
    pub intents: serenity::GatewayIntents,
    pub voice_manager_arc: Arc<songbird::Songbird>
}

#[shuttle_runtime::async_trait]
impl shuttle_runtime::Service for AyayaDiscordBot {
    async fn bind(mut self, addr: std::net::SocketAddr) -> Result<(), shuttle_runtime::Error> {
        use std::future::IntoFuture;

        let serve = axum::serve(
            shuttle_runtime::tokio::net::TcpListener::bind(addr)
                .await
                .map_err(shuttle_runtime::CustomError::new)?,
            self.router,
        )
        .into_future();

        let mut client = serenity::ClientBuilder::new(self.discord.token, self.discord.intents)
            .voice_manager_arc(self.discord.voice_manager_arc)
            .framework(self.discord.framework)
            .await
            .map_err(shuttle_runtime::CustomError::new)?;

        tokio::select! {
            _ = client.start_autosharded() => {},
            _ = serve => {},
        };

        Ok(())
    }
}
