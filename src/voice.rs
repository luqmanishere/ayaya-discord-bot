use std::{
    sync::{atomic::AtomicUsize, Arc},
    time::Duration,
};

use serenity::{client::Context, framework::standard::{macros::command, Args, CommandResult}, model::{channel::Message, id::ChannelId, misc::Mentionable}, prelude::*, utils::MessageBuilder};

use songbird::{
    input::{restartable::Restartable, Input},
    Event,
};
use tracing::info;

use crate::utils;
use crate::utils::check_msg;
use crate::{utils::get_manager, voice_events::*};

#[command]
#[description("Deafens Ayaya. She knows how to read lips, you know.")]
async fn deafen(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let handler_lock = match manager.get(guild_id) {
        Some(handler) => handler,
        None => {
            check_msg(msg.reply(ctx, "Not in a voice channel").await);

            return Ok(());
        }
    };

    let mut handler = handler_lock.lock().await;

    if handler.is_deaf() {
        check_msg(msg.channel_id.say(&ctx.http, "Already deafened").await);
    } else {
        if let Err(e) = handler.deafen(true).await {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, format!("Failed: {:?}", e))
                    .await,
            );
        }

        check_msg(msg.channel_id.say(&ctx.http, "Deafened").await);
    }

    Ok(())
}

#[command]
#[description("Joins the voice channel the user is currently in. PARTY TIME!")]
#[only_in(guilds)]
async fn join(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();

    let channel_id = guild
        .voice_states
        .get(&msg.author.id)
        .and_then(|voice_state| voice_state.channel_id);

    let connect_to = match channel_id {
        Some(channel) => channel,
        None => {
            check_msg(msg.reply(ctx, "You are not in a voice channel").await);

            return Ok(());
        }
    };

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;
    #[allow(unused_assignments)]
    let mut joined = false;
    let call = match manager.get(guild_id) {
        Some(handle_lock) => {
            joined = true;
            let handler = handle_lock.lock().await;
            ChannelId(handler.current_channel().unwrap().0)
        }
        None => {
            joined = false;
            ChannelId::default()
        }
    };

    if !joined {
        // TODO Prevent from joining channels if already in a channel
        let (handle_lock, success) = manager.join(guild_id, connect_to).await;

        if let Ok(_channel) = success {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, &format!("Joined {}", connect_to.mention()))
                    .await,
            );

            let chan_id = msg.channel_id;

            let mut handle = handle_lock.lock().await;

            // TODO Add event to send message on track start
            // TODO Add event to detect inactivity
            handle.add_global_event(
                Event::Periodic(Duration::from_secs(60), None),
                SongAfter60 {
                    channel_id: chan_id,
                    counter: Arc::new(AtomicUsize::new(0)),
                    guild_id: guild.id,
                    ctx: ctx.clone(),
                },
            );

            let send_http = ctx.http.clone();

            handle.add_global_event(
                Event::Periodic(Duration::from_secs(60), None),
                ChannelDurationNotifier {
                    chan_id,
                    count: Default::default(),
                    http: send_http,
                },
            );
        } else {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, "Error joining the channel")
                    .await,
            );
        }
    } else {
        let channel_name = call.name(&ctx.cache).await.unwrap();
        info!("Already in a channel {}, not joining", channel_name);
        check_msg(
            msg.channel_id
                .say(
                    &ctx.http,
                    format!("Already in voice channel \"{}\"", call.mention()),
                )
                .await,
        )
    }

    Ok(())
}

#[command]
#[description("Leaves the current voice channel. Ever wonder what happens to Ayaya then?")]
#[only_in(guilds)]
async fn leave(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();
    let has_handler = manager.get(guild_id).is_some();

    if has_handler {
        if let Err(e) = manager.remove(guild_id).await {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, format!("Failed: {:?}", e))
                    .await,
            );
        }

        check_msg(msg.channel_id.say(&ctx.http, "Left voice channel").await);
    } else {
        check_msg(msg.reply(ctx, "Not in a voice channel").await);
    }

    Ok(())
}

#[command]
#[description("Mutes Ayaya. Mmmhh mmhh mmmhhh????")]
#[only_in(guilds)]
async fn mute(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    let handler_lock = match manager.get(guild_id) {
        Some(handler) => handler,
        None => {
            check_msg(msg.reply(ctx, "Not in a voice channel").await);

            return Ok(());
        }
    };

    let mut handler = handler_lock.lock().await;

    if handler.is_mute() {
        check_msg(msg.channel_id.say(&ctx.http, "Already muted").await);
    } else {
        if let Err(e) = handler.mute(true).await {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, format!("Failed: {:?}", e))
                    .await,
            );
        }

        check_msg(msg.channel_id.say(&ctx.http, "Now muted").await);
    }

    Ok(())
}

#[command]
#[aliases("p")]
#[description("Plays music from YT url or search term. We are getting help from a higher being...")]
#[usage("<url/search term>")]
#[example("ayaya intensifies")]
#[only_in(guilds)]
async fn play(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let url = match args.single::<String>() {
        Ok(url) => url,
        Err(_) => {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, "Must provide a YT URL or a search term")
                    .await,
            );

            return Ok(());
        }
    };

    let mut search_yt = false;
    if !url.starts_with("http") {
        search_yt = true;
    }

    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = utils::get_manager(ctx).await;

    // Lock the manager to insert the audio into the queue if in voice channel
    if let Some(handler_lock) = manager.get(guild_id) {
        // let mut handler = handler_lock.lock().await;

        // Here, we use lazy restartable sources to make sure that we don't pay
        // for decoding, playback on tracks which aren't actually live yet.
        // Refactor this into functions later
        if !search_yt {
            let source = match Restartable::ytdl(url, true).await {
                Ok(source) => source,
                Err(why) => {
                    println!("Err starting source: {:?}", why);

                    check_msg(msg.channel_id.say(&ctx.http, "Error sourcing ffmpeg").await);

                    return Ok(());
                }
            };

            insert_source_with_message(source, handler_lock, msg, ctx).await;
        } else {
            let source = match Restartable::ytdl_search(url, true).await {
                Ok(source) => source,
                Err(why) => {
                    println!("Err starting source: {:?}", why);

                    check_msg(msg.channel_id.say(&ctx.http, "Error sourcing ffmpeg").await);

                    return Ok(());
                }
            };

            insert_source_with_message(source, handler_lock, msg, ctx).await;
        }

        // Queue the sources
    } else {
        check_msg(
            msg.channel_id
                .say(&ctx.http, "Not in a voice channel to play in")
                .await,
        );
    }

    Ok(())
}

#[command]
#[description(
    "Skips the currently playing song. Ayaya wonders why you abandoned your summon so easily."
)]
#[only_in(guilds)]
async fn skip(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        let song_name = queue.current().unwrap().metadata().title.clone().unwrap();
        let _ = queue.skip();

        check_msg(
            msg.channel_id
                .say(
                    &ctx.http,
                    format!(
                        "Skipped `{}` - {} left in queue.",
                        song_name,
                        queue.len() - 1
                    ),
                )
                .await,
        );
    } else {
        check_msg(
            msg.channel_id
                .say(&ctx.http, "Not in a voice channel to play in")
                .await,
        );
    }

    Ok(())
}

#[command]
#[aliases("q")]
#[description("Shows the queue. The only kind of acceptable spoiler")]
#[only_in(guilds)]
async fn queue(ctx: &Context, msg: &Message) -> CommandResult {
    // TODO Implement queue viewing
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = get_manager(ctx).await;

    // Check if in channel
    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        let tracks = queue.current_queue();
        let mut names = MessageBuilder::new();
        let mut i = 1;
        // TODO use message builder
        for track in tracks {
            names.push(format!("{}. ", i).as_str());
            names.push(format!("{} ({})\n", track.metadata().title.as_ref().unwrap(), track.metadata().channel.as_ref().unwrap()));
            i += 1;
        }
        check_msg(
            msg.channel_id
                .say(&ctx.http, format!("In Queue:\n```prolog\n{}```", names))
                .await,
        );
    } else {
        check_msg(
            msg.channel_id
                .say(&ctx.http, "Not in a voice channel to play in")
                .await,
        );
    }
    //TODO check for

    Ok(())
}

#[command]
#[description("Pause the party. Time is frozen in this bubble universe.")]
#[only_in(guilds)]
async fn pause(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        let song_name = queue.current().unwrap().metadata().title.clone().unwrap();
        let _ = queue.pause();

        check_msg(
            msg.channel_id
                .say(&ctx.http, format!("{} - paused", song_name))
                .await,
        );
    } else {
        check_msg(
            msg.channel_id
                .say(&ctx.http, "Not in a voice channel to play in")
                .await,
        );
    }

    Ok(())
}

#[command]
#[description("Resume the party. You hear a wind up sound as time speeds up.")]
#[only_in(guilds)]
async fn resume(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        let song_name = queue.current().unwrap().metadata().title.clone().unwrap();
        let _ = queue.resume();

        check_msg(
            msg.channel_id
                .say(&ctx.http, format!("{} - resumed", song_name))
                .await,
        );
    } else {
        check_msg(
            msg.channel_id
                .say(&ctx.http, "Not in a voice channel to play in")
                .await,
        );
    }

    Ok(())
}

#[command]
#[description("Stop all music and clear the queue. Will you stop by again?")]
#[only_in(guilds)]
async fn stop(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let handler = handler_lock.lock().await;
        let queue = handler.queue();
        let _ = queue.stop();

        check_msg(msg.channel_id.say(&ctx.http, "Queue cleared.").await);
    } else {
        check_msg(
            msg.channel_id
                .say(&ctx.http, "Not in a voice channel to play in")
                .await,
        );
    }

    Ok(())
}

#[command]
#[description("Undeafens the bot. Finally Ayaya pulls out her earplugs.")]
#[only_in(guilds)]
async fn undeafen(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;
        if let Err(e) = handler.deafen(false).await {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, format!("Failed: {:?}", e))
                    .await,
            );
        }

        check_msg(msg.channel_id.say(&ctx.http, "Undeafened").await);
    } else {
        check_msg(
            msg.channel_id
                .say(&ctx.http, "Not in a voice channel to undeafen in")
                .await,
        );
    }

    Ok(())
}

#[command]
#[description("Unmutes Ayaya. Poor Ayaya has been talking to herself unnoticed.")]
#[only_in(guilds)]
async fn unmute(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;
    let manager = songbird::get(ctx)
        .await
        .expect("Songbird Voice client placed in at initialisation.")
        .clone();

    if let Some(handler_lock) = manager.get(guild_id) {
        let mut handler = handler_lock.lock().await;
        if let Err(e) = handler.mute(false).await {
            check_msg(
                msg.channel_id
                    .say(&ctx.http, format!("Failed: {:?}", e))
                    .await,
            );
        }

        check_msg(msg.channel_id.say(&ctx.http, "Unmuted").await);
    } else {
        check_msg(
            msg.channel_id
                .say(&ctx.http, "Not in a voice channel to unmute in")
                .await,
        );
    }

    Ok(())
}

#[command]
#[aliases("np")]
#[description(
    "Shows what song is currently playing. Ayaya is really knows everything about herself."
)]
#[usage("")]
#[example("")]
#[only_in(guilds)]
async fn nowplaying(ctx: &Context, msg: &Message) -> CommandResult {
    let guild = msg.guild(&ctx.cache).await.unwrap();
    let guild_id = guild.id;

    let manager = get_manager(ctx).await;

    if let Some(handler) = manager.get(guild_id) {
        let handler = handler.lock().await;
        match handler.queue().current() {
            Some(track) => {
                let song_name = track.metadata().title.clone().unwrap();
                let channel_name = track.metadata().channel.clone().unwrap();

                check_msg(
                    msg.channel_id
                        .say(
                            &ctx.http,
                            format!("Now playing: `{} ({})`", song_name, channel_name),
                        )
                        .await,
                );
            }
            None => check_msg(
                msg.channel_id
                    .say(&ctx.http, "```prolog\nNothing is queued```")
                    .await,
            ),
        };
    } else {
        check_msg(
            msg.channel_id
                .say(&ctx.http, "Not in a voice channel to play in")
                .await,
        );
    }

    Ok(())
}

async fn insert_source_with_message(
    source: Restartable,
    handler_lock: Arc<Mutex<songbird::Call>>,
    msg: &Message,
    ctx: &Context,
) {
    let mut handler = handler_lock.lock().await;

    let song: Input = source.into();
    let song_name = song.metadata.title.clone().unwrap();
    let channel_name = song.metadata.channel.clone().unwrap();
    handler.enqueue_source(song);
    check_msg(
        msg.channel_id
            .say(
                &ctx.http,
                format!(
                    "Added `{} ({})` to queue: position {}",
                    song_name,
                    &channel_name,
                    handler.queue().len()
                ),
            )
            .await,
    );
}
