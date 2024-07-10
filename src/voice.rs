use std::{
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
use tracing::{error, info, log::warn};

// Imports within the crate
use crate::{
    utils::{
        self, check_msg, create_search_interaction, get_manager, metadata_to_embed, yt_search,
        OptionExt,
    },
    voice_events::*,
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

    let manager = songbird::get(ctx.serenity_context())
        .await
        .wrap_err("Songbird Voice client placed in at initialisation.")?
        .clone();

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
                ctx.reply("You are not in a voice channel").await?;

                return Ok(());
            }
        };
        if let Some(guild_id) = user_voice_state.guild_id {
            if guild_id == ctx.guild_id().wrap_err("getting guild id from context")? {
                match user_voice_state.channel_id {
                    Some(channel_id) => channel_id,
                    None => {
                        ctx.reply("You are not in a voice channel").await?;

                        return Ok(());
                    }
                }
            } else {
                ctx.reply("You are not messaging from the right guild")
                    .await?;

                return Ok(());
            }
        } else {
            warn!(
                "Not in a guild, expected guild id {}",
                ctx.guild_id().wrap_err("getting guild id from ctx")?
            );
            ctx.reply("Cache error. Please rejoin the channel").await?;

            return Err(eyre!("Error in the cache: voice state guild_id is None"));
        }
    };

    let manager = get_manager(ctx.serenity_context()).await;

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
                    ctx.reply(format!("Joined {}", voice_channel_id.mention()))
                        .await?;
                }

                // TODO: Add event to send message on track start
                // TODO: Add event to detect inactivity

                // inactive counter bot
                call.add_global_event(
                    Event::Periodic(Duration::from_secs(60), None),
                    BotInactiveCounter {
                        channel_id: chat_channel_id,
                        counter: Arc::new(AtomicUsize::new(0)),
                        guild_id,
                        manager: get_manager(ctx.serenity_context()).await,
                        ctx: ctx.serenity_context().to_owned(),
                    },
                );
            }
            Err(e) => {
                error!("Error joining channel: {}", e);
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

    let manager = songbird::get(ctx.serenity_context())
        .await
        .wrap_err("Songbird Voice client placed in at initialisation.")?
        .clone();
    let has_handler = manager.get(guild_id).is_some();

    if has_handler {
        if let Err(e) = manager.remove(guild_id).await {
            check_msg(ctx.channel_id().say(ctx, format!("Failed: {:?}", e)).await);
        }

        check_msg(ctx.channel_id().say(ctx, "Left voice channel").await);
    } else {
        ctx.reply("Not in a voice channel").await?;
    }

    Ok(())
}

/// Mutes Ayaya. Mmmhh mmhh mmmhhh????
#[poise::command(slash_command, prefix_command, guild_only)]
async fn mute(ctx: Context<'_>) -> Result<()> {
    let guild_id = ctx.guild_id().wrap_err("get guild id from context")?;

    let manager = songbird::get(ctx.serenity_context())
        .await
        .wrap_err("Songbird Voice client placed in at initialisation.")?
        .clone();

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

    play_inner(ctx, url).await?;
    Ok(())
}

async fn play_inner(ctx: Context<'_>, url: String) -> Result<()> {
    // join a channel first
    join_helper(ctx, false).await?;

    let search_yt = if !url.starts_with("http") {
        // ctx.channel_id()
        //     .say(ctx, format!("Searching Youtube for :{}", url))
        //     .await?;
        info!("searching youtube for: {}", url);
        true
    } else {
        // ctx.channel_id()
        //     .say(ctx, format!("Playing the link: {}", url))
        //     .await?;
        info!("got link: {}", url);
        false
    };

    ctx.defer().await?;

    let guild_id = ctx.guild_id().wrap_err("get guild id from context")?;

    let manager = utils::get_manager(ctx.serenity_context()).await;

    // Lock the manager to insert the audio into the queue if in voice channel
    if let Some(handler_lock) = manager.get(guild_id) {
        // Here, we use lazy restartable sources to make sure that we don't pay
        // for decoding, playback on tracks which aren't actually live yet.
        // Refactor this into functions later

        // the above comment is preserved for history. currently youtube playback is handled
        // by songbird's struct

        let source = if !search_yt {
            YoutubeDl::new(ctx.data().http.clone(), url)
        } else {
            YoutubeDl::new_search(ctx.data().http.clone(), url)
        };
        insert_source_with_message(source, handler_lock, ctx).await?;
    } else {
        check_msg(
            ctx.channel_id()
                .say(ctx, "Not in a voice channel to play in")
                .await,
        );
    }

    Ok(())
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

    // let list = results_msg.build();
    // let _prompt = ctx.channel_id().say(ctx, list).await?;

    // let wait = msg
    //     .channel_id
    //     .await_reply(ctx)
    //     .author_id(msg.author.id.0)
    //     .timeout(std::time::Duration::from_secs(15))
    //     .await;

    // match wait {
    //     Some(msg) => {
    //         match msg.content.parse::<usize>() {
    //             Ok(picked) => {
    //                 info!("Option picked: {}", msg.content);
    //                 prompt
    //                     .edit(ctx, |m| m.content(format!("Option picked: {}", picked)))
    //                     .await?;

    //                 let selection: String = res_vec[(picked - 1)].clone();
    //                 let _metadata = utils::yt_search(&selection).await?;
    //                 // TODO Display information beautifully
    //             }
    //             Err(_) => {
    //                 warn!("Input can't be parsed into numbers: {}", msg.content);
    //                 prompt
    //                     .edit(ctx, |m| {
    //                         m.content(
    //                             "Ayaya told you to give her a number...not whatever you just gave.",
    //                         )
    //                     })
    //                     .await?;
    //             }
    //         };
    //     }
    //     None => {
    //         prompt
    //             .edit(ctx, |m| {
    //                 m.content("Timeout! Ayaya wants you to decide in 10 seconds, not 10 minutes")
    //             })
    //             .await?;
    //     }
    // }

    Ok(())
}

// async fn _search(term: String, ctx: &Context, original_msg: &Message) -> EyreResult<String> {
//     let mut prompt = original_msg
//         .channel_id
//         .say(&ctx.http, "Searching...This takes quite a while")
//         .await?;

//     let vec = yt_9search(&term).await.unwrap();

//     prompt.edit(ctx, |m| m.content("Compiling list")).await?;

//     let mut list = MessageBuilder::new();
//     list.push_line("Pick an option to queue:")
//         .push_line("```prolog");
//     let mut i = 1;
//     for line in &vec {
//         list.push_line(format!("{} : {}", i, line));
//         i += 1;
//     }
//     let list = list.push_line("```").build();

//     prompt.edit(ctx, |m| m.content(list)).await?;
//     let wait = original_msg
//         .channel_id
//         .await_reply(ctx)
//         .author_id(original_msg.author.id.0)
//         .timeout(std::time::Duration::from_secs(15))
//         .await;

//     match wait {
//         Some(msg) => match msg.content.parse::<usize>() {
//             Ok(picked) => {
//                 info!("Option picked: {}", msg.content);
//                 prompt
//                     .edit(ctx, |m| m.content(format!("Option picked: {}", picked)))
//                     .await?;

//                 Ok(vec[(picked - 1)].clone())
//             }
//             Err(_) => {
//                 warn!("Input can't be parsed into numbers: {}", msg.content);
//                 prompt
//                     .edit(ctx, |m| {
//                         m.content(
//                             "Ayaya told you to give her a number...not whatever you just gave.",
//                         )
//                     })
//                     .await?;
//                 Err(eyre!("Can't convert into an index"))
//             }
//         },
//         None => {
//             prompt
//                 .edit(ctx, |m| {
//                     m.content("Timeout! Ayaya wants you to decide in 10 seconds, not 10 minutes")
//                 })
//                 .await?;
//             Err(eyre!("No answer was received"))
//         }
//     }
// }

/// Skips the currently playing song. Ayaya wonders why you abandoned your summon so easily.
#[poise::command(slash_command, prefix_command, guild_only)]
async fn skip(ctx: Context<'_>) -> Result<()> {
    let guild_id = ctx.guild_id().wrap_err("get guild id from ctx")?;

    let manager = songbird::get(ctx.serenity_context())
        .await
        .wrap_err("Songbird Voice client placed in at initialisation.")?
        .clone();

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
        let _ = queue.skip();

        check_msg(
            ctx.channel_id()
                .say(
                    ctx,
                    format!(
                        "Skipped `{}` - {} left in queue.",
                        song_name,
                        queue.len() - 1
                    ),
                )
                .await,
        );
    } else {
        check_msg(ctx.channel_id().say(ctx, "Not in a voice channel.").await);
    }

    Ok(())
}

/// Shows the queue. The only kind of acceptable spoilers.
#[poise::command(slash_command, prefix_command, aliases("q"), guild_only)]
async fn queue(ctx: Context<'_>) -> Result<()> {
    // TODO Implement queue viewing
    // let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = ctx.guild_id().wrap_err("getting guild id")?;

    let manager = get_manager(ctx.serenity_context()).await;

    // Check if in channel
    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        let tracks = queue.current_queue();
        let mut lines = serenity::MessageBuilder::new();
        {
            let data = ctx.data();
            let metadata_lock = data.track_metadata.lock().unwrap();

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

    let manager = songbird::get(ctx.serenity_context())
        .await
        .wrap_err("Songbird Voice client placed in at initialisation.")?
        .clone();

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

    let manager = get_manager(ctx.serenity_context()).await.clone();

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

    let manager = get_manager(ctx.serenity_context()).await.clone();

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

    let manager = get_manager(ctx.serenity_context()).await.clone();

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
    let manager = songbird::get(ctx.serenity_context())
        .await
        .wrap_err("Songbird Voice client placed in at initialisation.")?
        .clone();

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

/// "Shows what song is currently playing. Ayaya is really knows everything about herself."
#[poise::command(slash_command, prefix_command, aliases("np"), guild_only)]
async fn nowplaying(ctx: Context<'_>) -> Result<()> {
    let guild_id = ctx.guild_id().wrap_err("get guild id from ctx")?;

    let manager = get_manager(ctx.serenity_context()).await;

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
                let song_name = metadata.title.clone().unwrap();
                let channel_name = metadata.channel.clone().unwrap();

                check_msg(
                    ctx.channel_id()
                        .say(
                            ctx,
                            format!("Now playing: `{} ({})`", song_name, channel_name),
                        )
                        .await,
                );
            }
            None => {
                check_msg(
                    ctx.channel_id()
                        .say(ctx, "```prolog\nNothing is playing```")
                        .await,
                );
            }
        };
    } else {
        check_msg(
            ctx.channel_id()
                .say(ctx, "Not in a voice channel to play in")
                .await,
        );
    }

    Ok(())
}

#[poise::command(slash_command, prefix_command, guild_only)]
async fn seek(ctx: Context<'_>, secs: u64) -> Result<()> {
    let guild_id = ctx.guild_id().wrap_err("get guild id from ctx")?;

    let manager = get_manager(ctx.serenity_context()).await;

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
                check_msg(
                    ctx.channel_id()
                        .say(ctx, "```prolog\nNothing is playing```")
                        .await,
                );
            }
        };
    } else {
        check_msg(ctx.channel_id().say(ctx, "Not in a voice channel.").await);
    }

    Ok(())
}

async fn insert_source_with_message(
    mut source: YoutubeDl,
    handler_lock: Arc<Mutex<songbird::Call>>,
    ctx: Context<'_>,
) -> Result<()> {
    let mut handler = handler_lock.lock().await;
    let data = ctx.data();

    let metadata = source
        .aux_metadata()
        .await
        .wrap_err("get metadata from the net")?;

    let track: songbird::tracks::Track = source.into();
    let track_uuid = track.uuid;

    {
        let mut metadata_lock = data.track_metadata.lock().unwrap();
        metadata_lock.insert(track_uuid, metadata.clone());
    }

    let track_handle = handler.enqueue(track).await;
    track_handle.add_event(
        Event::Track(songbird::TrackEvent::Play),
        TrackPlayNotifier {
            channel_id: ctx.channel_id(),
            metadata: metadata.clone(),
            http: ctx.serenity_context().http.clone(),
        },
    )?;
    // TODO: log added track
    let embed = metadata_to_embed(utils::EmbedOperation::AddToQueue, &metadata);
    ctx.send(poise::CreateReply::default().embed(embed)).await?;

    Ok(())
}
