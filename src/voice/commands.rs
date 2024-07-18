use std::{
    collections::HashMap,
    sync::{atomic::AtomicUsize, Arc},
    time::Duration,
};

use eyre::{eyre, Context as EyreContext, ContextCompat, Result};
use poise::serenity_prelude as serenity;
use serenity::{model::id::ChannelId, prelude::*, Mentionable};
use songbird::{
    input::{Compose, YoutubeDl},
    Event,
};
use thiserror::Error;
use tracing::{error, info, log::warn};

// Imports within the crate
use super::events::*;
use crate::{
    utils::{
        self, check_msg, create_search_interaction, error_embed, metadata_to_embed, yt_playlist,
        yt_search, OptionExt,
    },
    Context,
};

#[poise::command(
    slash_command,
    prefix_command,
    subcommands(
        "join",
        "play",
        "leave",
        "mute",
        "queue",
        "nowplaying",
        "unmute",
        "search",
        "skip",
        "pause",
        "resume",
        "stop",
        "undeafen",
        "seek",
        "deafen",
    ),
    aliases("m"),
    subcommand_required
)]
pub async fn music(ctx: Context<'_>) -> Result<()> {
    info!("called by {}", ctx.author());
    Ok(())
}

// TODO: reply to slash commands properly

/// Deafens Ayaya. She knows how to read lips, you know.
#[poise::command(slash_command, prefix_command, guild_only)]
async fn deafen(ctx: Context<'_>) -> Result<()> {
    let guild_id = ctx.guild_id().wrap_err("command ran in guild")?;

    let manager = &ctx.data().songbird;

    let handler_lock = match manager.get(guild_id) {
        Some(handler) => handler,
        None => {
            ctx.reply("Not in a voice channel").await?;

            return Ok(());
        }
    };

    let mut handler = handler_lock.lock().await;

    if handler.is_deaf() {
        ctx.reply("Already deafened").await?;
    } else {
        if let Err(e) = handler.deafen(true).await {
            error!("Failed to deafen: {e}");
            ctx.say(format!("Failed to deafen: {:?}", e)).await?;
        }

        ctx.say("Deafened").await?;
    }

    Ok(())
}

/// Joins the voice channel the user is currently in. PARTY TIME!
#[poise::command(slash_command, prefix_command, guild_only, aliases("j"))]
async fn join(ctx: Context<'_>) -> Result<()> {
    join_helper(ctx, true).await
}

async fn join_helper(ctx: Context<'_>, play_notify_flag: bool) -> Result<()> {
    let guild: serenity::Guild = ctx.guild().wrap_err("getting guild from context")?.clone();
    let chat_channel_id = ctx.channel_id();
    let user_voice_state: Option<&serenity::VoiceState> = guild.voice_states.get(&ctx.author().id);

    let connect_to = {
        let user_voice_state = match user_voice_state {
            Some(voice_state) => voice_state,
            None => {
                ctx.send(
                    poise::CreateReply::default()
                        .embed(error_embed(utils::EmbedOperation::ErrorNotInVoiceChannel)),
                )
                .await?;

                // TODO: replace with proper errors
                return Err(eyre!("Not in voice channel"));
            }
        };
        if let Some(guild_id) = user_voice_state.guild_id {
            if guild_id == ctx.guild_id().wrap_err("getting guild id from context")? {
                match user_voice_state.channel_id {
                    Some(channel_id) => channel_id,
                    None => {
                        ctx.send(
                            poise::CreateReply::default()
                                .embed(error_embed(utils::EmbedOperation::ErrorNotInVoiceChannel)),
                        )
                        .await?;

                        // TODO: replace with proper errors
                        return Err(eyre!("Not in voice channel"));
                    }
                }
            } else {
                // TODO: replace with embed
                ctx.reply("You are not messaging from the right guild")
                    .await?;

                return Ok(());
            }
        } else {
            warn!(
                "Not in a guild, expected guild id {}",
                ctx.guild_id().wrap_err("getting guild id from ctx")?
            );
            // TODO: replace with embed
            ctx.reply("Cache error. Please rejoin the channel").await?;

            return Err(eyre!("Error in the cache: voice state guild_id is None"));
        }
    };

    let manager = &ctx.data().songbird;

    let guild_id = ctx.guild_id().wrap_err("getting guild id from ctx")?;

    let joined;
    let voice_channel_id = match manager.get(guild_id) {
        Some(handle_lock) => {
            joined = true;
            let handler = handle_lock.lock().await;
            ChannelId::new(
                handler
                    .current_channel()
                    .wrap_err_with(|| {
                        format!("getting current joined channel in guild {}", guild_id)
                    })?
                    .0
                    .into(),
            )
        }
        None => {
            joined = false;
            connect_to
        }
    };

    if !joined {
        // TODO Prevent from joining channels if already in a channel
        let call_res = manager.join(guild_id, connect_to).await;

        match call_res {
            Ok(call) => {
                let mut call = call.lock().await;
                info!("joined channel id: {voice_channel_id} in guild {guild_id}",);
                if play_notify_flag {
                    // TODO: replace with embed
                    ctx.reply(format!("Joined {}", voice_channel_id.mention()))
                        .await?;
                }

                // bot should be unmuted and deafened
                call.mute(false).await?;
                call.deafen(true).await?;

                // TODO: Add event to detect inactivity

                // inactive counter bot
                call.add_global_event(
                    Event::Periodic(Duration::from_secs(60), None),
                    BotInactiveCounter {
                        channel_id: chat_channel_id,
                        counter: Arc::new(AtomicUsize::new(0)),
                        guild_id,
                        manager: ctx.data().songbird.clone(),
                        ctx: ctx.serenity_context().to_owned(),
                    },
                );
            }
            Err(e) => {
                error!("Error joining channel: {}", e);
                // TODO: replace with embed
                ctx.say("Unable to join voice channel").await?;
            }
        }
    } else {
        let channel_name = voice_channel_id
            .name(ctx)
            .await
            .wrap_err("getting channel name from id")?;
        warn!("Already in a channel {}, not joining", channel_name);
        if play_notify_flag {
            // TODO: replace with embed
            ctx.channel_id()
                .say(
                    ctx,
                    format!(
                        "Already in voice channel \"{}\"",
                        voice_channel_id.mention()
                    ),
                )
                .await?;
        }
    }
    Ok(())
}

/// Leaves the current voice channel. Ever wonder what happens to Ayaya then?
#[poise::command(slash_command, prefix_command, guild_only)]
async fn leave(ctx: Context<'_>) -> Result<()> {
    let guild_id = ctx.guild_id().wrap_err("getting guild id from context")?;

    let manager = &ctx.data().songbird;
    let has_handler = manager.get(guild_id).is_some();

    if has_handler {
        if let Err(e) = manager.remove(guild_id).await {
            // FIXME: wtf is this
            check_msg(ctx.channel_id().say(ctx, format!("Failed: {:?}", e)).await);
        }

        // TODO: replace with embeds
        check_msg(ctx.channel_id().say(ctx, "Left voice channel").await);
    } else {
        ctx.send(
            poise::CreateReply::default()
                .embed(error_embed(utils::EmbedOperation::ErrorNotInVoiceChannel)),
        )
        .await?;
    }

    Ok(())
}

/// Mutes Ayaya. Mmmhh mmhh mmmhhh????
#[poise::command(slash_command, prefix_command, guild_only)]
async fn mute(ctx: Context<'_>) -> Result<()> {
    let guild_id = ctx.guild_id().wrap_err("get guild id from context")?;

    let manager = &ctx.data().songbird;

    let handler_lock = match manager.get(guild_id) {
        Some(handler) => handler,
        None => {
            ctx.reply("Not in a voice channel").await?;

            return Ok(());
        }
    };

    let mut handler = handler_lock.lock().await;

    if handler.is_mute() {
        check_msg(ctx.channel_id().say(ctx, "Already muted").await);
    } else {
        if let Err(e) = handler.mute(true).await {
            check_msg(ctx.channel_id().say(ctx, format!("Failed: {:?}", e)).await);
        }

        ctx.say("Now muted").await?;
    }

    Ok(())
}

/// Plays music from YT url or search term. We are getting help from a higher being...
#[poise::command(slash_command, prefix_command, aliases("p"), guild_only)]
async fn play(
    ctx: Context<'_>,
    #[description = "A url or a search term for youtube"]
    #[min_length = 1]
    url: Vec<String>,
) -> Result<()> {
    // convert vec to a string
    let url = url.join(" ").trim().to_string();

    ctx.defer().await?;

    play_inner(ctx, url).await?;
    Ok(())
}

pub enum PlayParse {
    Search(String),
    Url(String),
    PlaylistUrl(String),
}

impl PlayParse {
    pub fn parse(input: &str) -> Self {
        if input.starts_with("http") {
            if input.contains("playlist") {
                return Self::PlaylistUrl(input.to_string());
            }

            Self::Url(input.to_string())
        } else {
            Self::Search(input.to_string())
        }
    }

    /// Handle the parsed input for play. Takes the poise context to facilitate communication
    pub async fn run(self, ctx: Context<'_>) -> Result<()> {
        let manager = ctx.data().songbird.clone();
        let guild_id = ctx.guild_id().wrap_err("get guild id from ctx")?;
        let calling_channel_id = ctx.channel_id();
        let call = manager.get(guild_id);
        match self {
            PlayParse::Search(search) => {
                info!("searching youtube for: {}", search);
                let source = YoutubeDl::new_search(ctx.data().http.clone(), search);
                match handle_single_play(call, calling_channel_id, source, ctx).await {
                    Ok(_) => {}
                    Err(_) => {
                        ctx.send(
                            poise::CreateReply::default()
                                .embed(error_embed(utils::EmbedOperation::ErrorNotInVoiceChannel)),
                        )
                        .await?;
                    }
                }
            }
            PlayParse::Url(url) => {
                info!("using provided link: {}", url);
                let source = YoutubeDl::new(ctx.data().http.clone(), url);
                match handle_single_play(call, calling_channel_id, source, ctx).await {
                    Ok(_) => {}
                    Err(_) => {
                        ctx.send(
                            poise::CreateReply::default()
                                .embed(error_embed(utils::EmbedOperation::ErrorNotInVoiceChannel)),
                        )
                        .await?;
                    }
                }
            }
            PlayParse::PlaylistUrl(playlist_url) => {
                ctx.reply("Handling playlist....").await?;

                // TODO: implement playlist loading
                let metadata_vec = yt_playlist(playlist_url).await?;

                let channel_id = ctx.channel_id();
                let call = manager.get(guild_id);

                for metadata in metadata_vec {
                    tokio::spawn(handle_from_playlist(
                        metadata,
                        ctx.data().http.clone(),
                        ctx.data().track_metadata.clone(),
                        call.clone(),
                        ctx.serenity_context().http.clone(),
                        channel_id,
                    ));
                }
            }
        }
        Ok(())
    }
}

async fn handle_from_playlist(
    metadata: utils::YoutubeMetadata,
    http: reqwest::Client,
    track_metadata: Arc<std::sync::Mutex<HashMap<uuid::Uuid, songbird::input::AuxMetadata>>>,
    call: Option<Arc<Mutex<songbird::Call>>>,
    serenity_http: Arc<serenity::Http>,
    calling_channel_id: ChannelId,
) -> Result<songbird::input::AuxMetadata> {
    // our ids are formatted into youtube links to prevent command line errors
    let youtube_link = format!("https://www.youtube.com/watch?v={}", metadata.youtube_id);

    let source = YoutubeDl::new(http.clone(), youtube_link);
    insert_source(
        source,
        track_metadata,
        call,
        serenity_http,
        calling_channel_id,
    )
    .await
}

async fn play_inner(ctx: Context<'_>, input: String) -> Result<()> {
    let input_type = PlayParse::parse(&input);

    // join a channel first
    join_helper(ctx, false).await?;

    // TODO: check if youtube url

    input_type.run(ctx).await
}

/// Search YT and get metadata
#[poise::command(slash_command, prefix_command)]
// #[usage("<search term>")]
// #[example("ayaya intensifies")]
async fn search(ctx: Context<'_>, search_term: Vec<String>) -> Result<()> {
    let term = search_term.join(" ");

    // reply or say in channel depending on command type
    match ctx {
        poise::Context::Application(ctx) => {
            ctx.reply(format!("Searching youtube for: {term}")).await?;
        }
        poise::Context::Prefix(ctx) => {
            ctx.channel_id()
                .say(ctx, format!("Searching youtube for: {term}"))
                .await?;
        }
    }
    ctx.defer().await?;

    // let songbird do the searching
    let search = yt_search(&term, Some(10))
        .await
        .wrap_err_with(|| eyre!("searching youtube for term: {}", &term))?;

    match create_search_interaction(ctx, search).await {
        Ok(youtube_id) => {
            play_inner(ctx, youtube_id).await?;
        }
        Err(e) => {
            error!("Error from interaction: {e}");
        }
    };

    Ok(())
}

/// Skips the currently playing song. Ayaya wonders why you abandoned your summon so easily.
#[poise::command(slash_command, prefix_command, guild_only)]
async fn skip(ctx: Context<'_>) -> Result<()> {
    let guild_id = ctx.guild_id().wrap_err("get guild id from ctx")?;

    let manager = &ctx.data().songbird;

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        let track_uuid = queue.current().unwrap().uuid();
        let song_metadata = {
            let metadata_lock = ctx.data().track_metadata.lock().unwrap();
            metadata_lock
                .get(&track_uuid)
                .wrap_err_with(|| eyre!("getting metadata for uuid: {}", track_uuid))?
                .clone()
        };
        queue.skip()?;

        let embed = metadata_to_embed(utils::EmbedOperation::SkipSong, &song_metadata, None);

        ctx.send(poise::CreateReply::default().embed(embed)).await?;
    } else {
        ctx.say("Not in a voice channel, Ayaya can't skip air bruh.")
            .await?;
    }

    Ok(())
}

/// Shows the queue. The only kind of acceptable spoilers.
#[poise::command(slash_command, prefix_command, aliases("q"), guild_only)]
async fn queue(ctx: Context<'_>) -> Result<()> {
    // TODO Implement queue viewing
    // let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = ctx.guild_id().wrap_err("getting guild id")?;

    let manager = &ctx.data().songbird;

    // Check if in channel
    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        let tracks = queue.current_queue();
        let mut lines = serenity::MessageBuilder::new();
        {
            let data = ctx.data();
            let metadata_lock = data.track_metadata.lock().unwrap();

            // TODO: replace with embed
            if tracks.is_empty() {
                lines.push_line("# Nothing in queue");
            } else {
                lines.push_line("# Queue");
            }
            for (i, track) in tracks.iter().enumerate() {
                let track_uuid = track.uuid();
                let metadata = metadata_lock
                    .get(&track_uuid)
                    .wrap_err("getting track metadata")?;

                lines.push_quote_line(format!(
                    "{}. {} ({})",
                    i + 1,
                    metadata.title.as_ref().unwrap(),
                    metadata.channel.as_ref().unwrap()
                ));
            }
        }

        let embed = serenity::CreateEmbed::new()
            .colour(serenity::Colour::MEIBE_PINK)
            .description(lines.to_string());

        let msg = serenity::CreateMessage::new().embed(embed);

        check_msg(ctx.channel_id().send_message(ctx, msg).await);
    } else {
        check_msg(
            ctx.channel_id()
                .say(ctx, "Not in a voice channel to play in")
                .await,
        );
    }
    //TODO check for

    Ok(())
}

/// Pause the party. Time is frozen in this bubble universe."
#[poise::command(slash_command, prefix_command, guild_only)]
async fn pause(ctx: Context<'_>, _args: String) -> Result<()> {
    let guild_id = ctx.guild_id().wrap_err("get guild id from ctx")?;

    let manager = &ctx.data().songbird;

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        let track_uuid = queue.current().unwrap().uuid();
        let song_name = {
            let metadata_lock = ctx.data().track_metadata.lock().unwrap();
            metadata_lock
                .get(&track_uuid)
                .wrap_err_with(|| eyre!("getting metadata for uuid: {}", track_uuid))?
                .title
                .clone()
                .unwrap_or_unknown()
        };
        queue.pause()?;

        check_msg(
            ctx.channel_id()
                .say(ctx, format!("{} - paused", song_name))
                .await,
        );
    } else {
        check_msg(
            ctx.channel_id()
                .say(ctx, "Not in a voice channel to play in")
                .await,
        );
    }

    Ok(())
}

/// Resume the party. You hear a wind up sound as time speeds up.
#[poise::command(slash_command, prefix_command, guild_only)]
async fn resume(ctx: Context<'_>) -> Result<()> {
    let guild_id = ctx.guild_id().wrap_err("getting guild id from ctx")?;

    let manager = &ctx.data().songbird;

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        let track_uuid = queue.current().unwrap().uuid();
        let song_name = {
            let metadata_lock = ctx.data().track_metadata.lock().unwrap();
            metadata_lock
                .get(&track_uuid)
                .wrap_err_with(|| eyre!("getting metadata for uuid: {}", track_uuid))?
                .title
                .clone()
                .unwrap_or_unknown()
        };
        queue.resume()?;

        check_msg(
            ctx.channel_id()
                .say(ctx, format!("{} - resumed", song_name))
                .await,
        );
    } else {
        check_msg(
            ctx.channel_id()
                .say(ctx, "Not in a voice channel to play in")
                .await,
        );
    }

    Ok(())
}

/// Stop all music and clear the queue. Will you stop by again?
#[poise::command(prefix_command, slash_command, guild_only)]
async fn stop(ctx: Context<'_>) -> Result<()> {
    let guild_id = ctx.guild_id().wrap_err("getting guild id from ctx")?;

    let manager = &ctx.data().songbird;

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        queue.stop();

        check_msg(ctx.channel_id().say(ctx, "Queue cleared.").await);
    } else {
        check_msg(
            ctx.channel_id()
                .say(ctx, "Not in a voice channel to play in")
                .await,
        );
    }

    Ok(())
}

// TODO: implement skipping a certain track

// #[command]
// #[aliases("d")]
// #[description("Delete song from queue. Being able to make things go *poof* makes you feel like a Kami-sama, right?")]
// async fn delete(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
//     let guild = msg.guild(&ctx).await.unwrap();
//     let guild_id = guild.id;

//     let manager = get_manager(ctx).await;

//     if let Some(handler_lock) = manager.get(guild_id) {
//         // If not empty, remove the songs
//         if !args.is_empty() {
//             let handler = handler_lock.lock().await;
//             let queue = handler.queue();
//             if let Ok(index) = args.single::<usize>() {
//                 if index != 1 {
//                     let index = index - 1;
//                     if let Some(track) = queue.current_queue().get(index) {
//                         let song_name = track.metadata().title.clone().unwrap();
//                         let channel_name = track.metadata().title.clone().unwrap();
//                         check_msg(
//                             msg.channel_id
//                                 .say(
//                                     &ctx.http,
//                                     format!(
//                                         "Removing `{} ({})` from position {}",
//                                         song_name,
//                                         channel_name,
//                                         index + 1
//                                     ),
//                                 )
//                                 .await,
//                         );
//                         queue.dequeue(index);
//                     }
//                 } else {
//                     check_msg(
//                         msg.channel_id
//                             .say(&ctx.http, "Sorry, Ayaya can't delete what she is playing.")
//                             .await,
//                     );
//                 }
//             }
//         } else {
//             // Tell them to give arguments
//             check_msg(
//                 msg.channel_id
//                     .say(
//                         &ctx.http,
//                         "Ayaya needs to know which song you want to delete, baka.",
//                     )
//                     .await,
//             );
//         }
//     } else {
//         check_msg(
//             msg.channel_id
//                 .say(
//                     &ctx.http,
//                     "Ayaya is not in a voice channel, hence she has nothing to delete.",
//                 )
//                 .await,
//         );
//     }

//     Ok(())
// }

/// Undeafens the bot. Finally, Ayaya pulls out her earplugs.
#[poise::command(slash_command, prefix_command, guild_only)]
async fn undeafen(ctx: Context<'_>) -> Result<()> {
    let guild_id = ctx.guild_id().wrap_err("getting guild id from ctx")?;

    let manager = &ctx.data().songbird;

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;
        if let Err(e) = handler.deafen(false).await {
            check_msg(ctx.channel_id().say(ctx, format!("Failed: {:?}", e)).await);
        }

        check_msg(ctx.channel_id().say(ctx, "Undeafened").await);
    } else {
        check_msg(
            ctx.channel_id()
                .say(ctx, "Ayaya is not in a voice channel to undeafen in.")
                .await,
        );
    }

    Ok(())
}

/// Unmutes Ayaya. Poor Ayaya has been talking to herself unnoticed.
#[poise::command(slash_command, prefix_command, guild_only, aliases("um"))]
async fn unmute(ctx: Context<'_>) -> Result<()> {
    let guild_id = ctx.guild_id().wrap_err("get guild id from ctx")?;
    let manager = &ctx.data().songbird;

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;
        if let Err(e) = handler.mute(false).await {
            check_msg(ctx.channel_id().say(ctx, format!("Failed: {:?}", e)).await);
        }

        check_msg(ctx.channel_id().say(ctx, "Unmuted").await);
    } else {
        check_msg(
            ctx.channel_id()
                .say(ctx, "Not in a voice channel to unmute in")
                .await,
        );
    }

    Ok(())
}

/// "Shows what song is currently playing. Ayaya really knows everything about herself."
#[poise::command(slash_command, prefix_command, aliases("np"), guild_only)]
async fn nowplaying(ctx: Context<'_>) -> Result<()> {
    let guild_id = ctx.guild_id().wrap_err("get guild id from ctx")?;

    let manager = &ctx.data().songbird;

    if let Some(handler) = manager.get(guild_id) {
        let handler = handler.lock().await;
        match handler.queue().current() {
            Some(track) => {
                let data = ctx.data();
                let track_uuid = track.uuid();
                let track_state = track.get_info().await?;
                let metadata = {
                    let lock = data.track_metadata.lock().unwrap();
                    lock.get(&track_uuid)
                        .wrap_err("expect track to exist")?
                        .clone()
                };

                ctx.send(poise::CreateReply::default().embed(metadata_to_embed(
                    utils::EmbedOperation::NowPlaying,
                    &metadata,
                    Some(&track_state),
                )))
                .await?;
            }
            None => {
                ctx.send(poise::CreateReply::default().embed(metadata_to_embed(
                    utils::EmbedOperation::NowPlaying,
                    &songbird::input::AuxMetadata::default(),
                    None,
                )))
                .await?;
            }
        };
    } else {
        ctx.send(
            poise::CreateReply::default()
                .embed(error_embed(utils::EmbedOperation::ErrorNotInVoiceChannel)),
        )
        .await?;
    }

    Ok(())
}

#[poise::command(slash_command, prefix_command, guild_only)]
async fn seek(ctx: Context<'_>, secs: u64) -> Result<()> {
    let guild_id = ctx.guild_id().wrap_err("get guild id from ctx")?;

    let manager = &ctx.data().songbird;

    if let Some(handler) = manager.get(guild_id) {
        let handler = handler.lock().await;
        match handler.queue().current() {
            Some(track) => {
                let data = ctx.data();
                let track_uuid = track.uuid();
                let metadata = {
                    let lock = data.track_metadata.lock().unwrap();
                    lock.get(&track_uuid)
                        .wrap_err("expect track to exist")?
                        .clone()
                };
                track
                    .seek(std::time::Duration::from_secs(secs))
                    .result()
                    // .wrap_err("seeking track")
                 ?;
                let song_name = metadata.title.clone().unwrap();
                let channel_name = metadata.channel.clone().unwrap();

                // TODO: express in embed
                check_msg(
                    ctx.channel_id()
                        .say(
                            ctx,
                            format!(
                                "Seek track: `{} ({})` to {} seconds",
                                song_name, channel_name, secs
                            ),
                        )
                        .await,
                );
            }
            None => {
                ctx.send(
                    poise::CreateReply::default()
                        .embed(error_embed(utils::EmbedOperation::ErrorNotPlaying)),
                )
                .await?;
            }
        };
    } else {
        ctx.send(
            poise::CreateReply::default()
                .embed(error_embed(utils::EmbedOperation::ErrorNotInVoiceChannel)),
        )
        .await?;
    }

    Ok(())
}

/// Inserts a youtube source, sets events and notifies the calling channel
async fn handle_single_play(
    call: Option<Arc<Mutex<songbird::Call>>>,
    calling_channel_id: ChannelId,
    source: YoutubeDl,
    ctx: Context<'_>,
) -> Result<()> {
    let metadata = insert_source(
        source,
        ctx.data().track_metadata.clone(),
        call,
        ctx.serenity_context().http.clone(),
        calling_channel_id,
    )
    .await?;

    let embed = metadata_to_embed(utils::EmbedOperation::AddToQueue, &metadata, None);
    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}

/// Process the given source, obtain its metadata and handle track insertion with events. This
/// function is made to be used with tokio::spawn
async fn insert_source(
    mut source: YoutubeDl,
    track_metadata: Arc<std::sync::Mutex<HashMap<uuid::Uuid, songbird::input::AuxMetadata>>>,
    call: Option<Arc<Mutex<songbird::Call>>>,
    serenity_http: Arc<serenity::Http>,
    calling_channel_id: ChannelId,
) -> Result<songbird::input::AuxMetadata> {
    match source.aux_metadata().await {
        Ok(metadata) => {
            let track: songbird::tracks::Track = source.into();
            let track_uuid = track.uuid;

            {
                let mut metadata_lock = track_metadata.lock().unwrap();
                metadata_lock.insert(track_uuid, metadata.clone());
            }

            // queue the next song few seconds before current song ends
            let preload_time = if let Some(duration) = metadata.duration {
                duration.checked_sub(std::time::Duration::from_secs(8))
            } else {
                None
            };

            if let Some(handler_lock) = &call {
                let mut handler = handler_lock.lock().await;
                let track_handle = handler.enqueue_with_preload(track, preload_time);

                let serenity_http_clone = serenity_http.clone();
                track_handle
                    .add_event(
                        Event::Track(songbird::TrackEvent::Play),
                        TrackPlayNotifier {
                            channel_id: calling_channel_id,
                            metadata: metadata.clone(),
                            http: serenity_http_clone,
                        },
                    )
                    .unwrap();
                info!(
                    "Added track {} ({}) to channel {calling_channel_id}",
                    metadata.title.clone().unwrap_or_unknown(),
                    metadata.channel.clone().unwrap_or_unknown()
                );
                // Logging added playlist in discord will be annoyying af
                Ok(metadata)
            } else {
                error!("Call does not exist...");
                Err(eyre!("Call does not exist..."))
            }
        }
        Err(e) => {
            let err = format!("Unable to get metadata from youtube {e}");
            error!(err);
            Err(eyre!(err))
        }
    }
}

#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum MusicCommandError {
    #[error("no calls joined in guild {0}")]
    BotCallNotJoined(serenity::GuildId),
}
