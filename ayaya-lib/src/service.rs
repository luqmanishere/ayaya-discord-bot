use std::sync::Arc;

use axum::{async_trait, http::StatusCode};
use axum::{extract::FromRequestParts, http::request::Parts};
use axum_auth::{AuthBasicCustom, Rejection};
use miette::IntoDiagnostic;
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
    pub voice_manager_arc: Arc<songbird::Songbird>,
}

use anyhow::Result;
impl AyayaDiscordBot {
    pub async fn local_bind(self, addr: std::net::SocketAddr) -> miette::Result<()> {
        use std::future::IntoFuture;

        let serve = axum::serve(
            tokio::net::TcpListener::bind(addr)
                .await
                .into_diagnostic()?,
            self.router,
        )
        .into_future();

        let mut client = serenity::ClientBuilder::new(self.discord.token, self.discord.intents)
            .voice_manager_arc(self.discord.voice_manager_arc)
            .framework(self.discord.framework)
            .await
            .into_diagnostic()?;

        tokio::select! {
            _ = client.start_autosharded() => {},
            _ = serve => {},
        };

        Ok(())
    }
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

/// Your custom basic auth returning a 401 Unauthorized for compliance with Grafana Cloud expectations
pub struct MetricsBasicAuth(pub (String, Option<String>));

// this is where you define your custom options
impl AuthBasicCustom for MetricsBasicAuth {
    const ERROR_CODE: StatusCode = StatusCode::UNAUTHORIZED; // <-- define custom status code here
    const ERROR_OVERWRITE: Option<&'static str> = None; // <-- define overwriting message here

    fn from_header(contents: (String, Option<String>)) -> Self {
        Self(contents)
    }
}

// this is just boilerplate, copy-paste this
#[async_trait]
impl<B> FromRequestParts<B> for MetricsBasicAuth
where
    B: Send + Sync,
{
    type Rejection = Rejection;

    async fn from_request_parts(parts: &mut Parts, _: &B) -> Result<Self, Self::Rejection> {
        Self::decode_request_parts(parts)
    }
}
