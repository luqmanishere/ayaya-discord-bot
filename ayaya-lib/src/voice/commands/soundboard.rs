use std::{io::Read, sync::Arc};

use error::SoundboardError;
use poise::serenity_prelude as serenity;
use snafu::ResultExt;
use tokio::sync::Mutex;

use crate::{
    CommandResult, Context,
    error::{
        BotError, DataManagerSnafu, DownloadAttachmentSnafu, ExternalCommandSnafu,
        FilesystemAccessSnafu, GeneralSerenitySnafu, IoSnafu,
    },
    utils::GuildInfo,
    voice::{
        error::MusicCommandError,
        utils::{EmbedOperation, embed_template},
    },
};

use super::join;

/// Upload a sound to the server. Guaranteed to accept valid MP3 files.
#[poise::command(
    slash_command,
    prefix_command,
    guild_only,
    ephemeral,
    category = "Soundboard"
)]
pub async fn upload_sound(
    ctx: Context<'_>,
    #[description = "A memorable description for the sound"] description: String,
    file: serenity::Attachment,
    #[description = "Whether others can use this sound. true or false."] public: Option<bool>,
) -> CommandResult {
    // TODO: logs
    ctx.defer_ephemeral().await.context(GeneralSerenitySnafu)?;
    let sound_manager = ctx.data().data_manager.sounds();
    let guild_id = GuildInfo::guild_id_or_0(ctx);
    let user_id = ctx.author().id;
    // if not specified, the default is true
    let public = public.unwrap_or(true);

    // show the notice if its a public upload without the policy acceptance
    if public
        && !sound_manager
            .get_user_public_upload_policy(&user_id)
            .await
            .context(DataManagerSnafu)?
    {
        let result = create_public_upload_notice(ctx).await?;
        ctx.defer_ephemeral().await.context(GeneralSerenitySnafu)?;

        match result {
            Some(true) => {
                sound_manager
                    .set_user_public_upload_policy(&user_id, true)
                    .await
                    .context(DataManagerSnafu)?;
                ctx.reply("Okay, we won't bother you with that anymore.")
                    .await
                    .context(GeneralSerenitySnafu)?;
            }
            Some(false) => {
                sound_manager
                    .set_user_public_upload_policy(&user_id, false)
                    .await
                    .context(DataManagerSnafu)?;
                return Err(SoundboardError::PolicyDeclined.into());
            }
            None => {
                ctx.reply("Timeout. Upload canceled.")
                    .await
                    .context(GeneralSerenitySnafu)?;
                return Err(SoundboardError::NoticeTimeout.into());
            }
        }
    }

    ctx.defer_ephemeral().await.context(GeneralSerenitySnafu)?;

    let sound_dir = ctx.data().data_dir.join("sounds");
    std::fs::create_dir_all(&sound_dir).context(FilesystemAccessSnafu {
        path: sound_dir.clone(),
    })?;

    // download, hash the original file, then
    let filename = &file.filename;
    let downloaded_file = match file.download().await {
        Ok(down) => {
            tracing::info!("downloaded file from discord");
            down
        }
        Err(e) => {
            tracing::error!("error downloading file");
            return Err(e).context(DownloadAttachmentSnafu);
        }
    };

    // hash the original file to get an identifier
    let mut hasher = sha1_smol::Sha1::new();
    hasher.update(&downloaded_file);
    let mut bytes = [0; 16];
    bytes.copy_from_slice(&hasher.digest().bytes()[..16]);
    let sound_id = uuid::Builder::from_sha1_bytes(bytes).into_uuid();

    // simplify ffmpeg usage by just using files
    let tempdir = tempfile::tempdir().expect("tempdir can be created");
    let temp_input_path = tempdir.path().join(filename);
    tracing::info!("downloading to {}", temp_input_path.display());
    std::fs::write(&temp_input_path, &downloaded_file).context(FilesystemAccessSnafu {
        path: &temp_input_path,
    })?;

    let outfile = tempdir.path().join(format!("{sound_id}.mp3"));

    let (mut recv, send) = std::io::pipe().expect("unable to create anonymous pipe");

    let mut command = tokio::process::Command::new("ffmpeg")
        .args([
            "-i",
            temp_input_path.display().to_string().as_str(),
            outfile.display().to_string().as_str(),
        ])
        .stdout(send.try_clone().expect("unable to clone pipe"))
        .stderr(send)
        .spawn()
        .context(ExternalCommandSnafu)?;

    let mut output = Vec::new();
    recv.read_to_end(&mut output).context(IoSnafu)?;
    tracing::debug!("ffmpeg{}", String::from_utf8(output).unwrap_or_default());

    let success = command
        .wait()
        .await
        .expect("error waiting for command")
        .success();

    if !success {
        ctx.reply("Unable to add sound, not an audio file?")
            .await
            .context(GeneralSerenitySnafu)?;
        return Err(BotError::MusicCommandError {
            source: MusicCommandError::SoundboardError {
                source: SoundboardError::NotAudioFile,
            },
        });
    }

    let final_file_path = sound_dir.join(format!("{sound_id}.mp3"));
    std::fs::copy(outfile, &final_file_path).expect("unable to copy file");
    tracing::info!("copied final output to: {}", final_file_path.display());

    sound_manager
        .add_sound(
            &ctx.author().id,
            guild_id,
            sound_id,
            description.clone(),
            Some(public),
        )
        .await
        .context(DataManagerSnafu)?;
    tracing::info!(
        "Added sound {description} with id {sound_id}, publicity {public:?} from user {}",
        ctx.author()
    );

    ctx.reply(format!("Added sound {description} with id {sound_id}"))
        .await
        .context(GeneralSerenitySnafu)?;
    Ok(())
}

/// Play an uploaded sound.
#[poise::command(slash_command, guild_only, prefix_command, category = "Soundboard")]
pub async fn play_sound(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_play_sound"]
    #[description = "The sound identifier. Refer to the autocomplete"]
    sound_id: uuid::Uuid,
) -> Result<(), BotError> {
    ctx.defer_ephemeral().await.context(GeneralSerenitySnafu)?;

    let guild_id = crate::utils::get_guild_id(ctx)?;

    if join::join_inner(ctx, false, true).await? {
        ctx.data()
            .linger_map
            .lock()
            .await
            .get_mut(&guild_id)
            .expect("joined channel without linger map")
            .store(true, std::sync::atomic::Ordering::Relaxed);
        tracing::debug!("set channel to linger");
    };

    let manager = ctx.data().songbird.clone();
    let call = manager.get(guild_id).expect("exists");
    let path = ctx
        .data()
        .data_dir
        .join("sounds")
        .join(format!("{sound_id}.mp3"));
    tracing::info!("path: {}", path.display());
    let input = songbird::input::File::new(path);

    {
        let mut lock = call.lock().await;
        lock.play(input.into());
    }

    // TODO: which sound is played?
    let sound_data = ctx
        .data()
        .data_manager
        .sounds()
        .get_sound_details(sound_id)
        .await
        .expect("valid sound was not found");
    create_sound_repeat(ctx, &ctx.author().id, sound_data, call).await?;
    Ok(())
}

/// Rename a sound that you uploaded
#[poise::command(slash_command, guild_only, prefix_command, category = "Soundboard")]
pub async fn rename_sound(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_rename_sound"]
    #[description = "The sound identifier. Refer to the autocomplete"]
    sound_id: uuid::Uuid,
    new_description: String,
) -> Result<(), BotError> {
    ctx.defer_ephemeral().await.context(GeneralSerenitySnafu)?;

    let old_sound = ctx
        .data()
        .data_manager
        .sounds()
        .get_sound_details(sound_id)
        .await
        .expect("sound exists");

    ctx.data()
        .data_manager
        .sounds()
        .rename_sound(sound_id, new_description.clone())
        .await
        .context(DataManagerSnafu)?;

    // TODO: embed
    ctx.reply(format!(
        "Renamed {} to {new_description}",
        old_sound.sound_name
    ))
    .await
    .context(GeneralSerenitySnafu)?;

    Ok(())
}

async fn autocomplete_play_sound(
    ctx: Context<'_>,
    partial: &str,
) -> Vec<serenity::AutocompleteChoice> {
    let user = ctx.author();
    let sound_manager = ctx.data().data_manager.sounds();
    let partial = partial.to_lowercase();

    let sounds = sound_manager
        .get_user_sounds_and_public(&user.id)
        .await
        .unwrap_or_default();

    sounds
        .iter()
        .filter(|e| e.sound_name.to_lowercase().contains(&partial))
        .map(|e| serenity::AutocompleteChoice::new(e.sound_name.clone(), e.sound_id.to_string()))
        .collect()
}

async fn autocomplete_rename_sound(
    ctx: Context<'_>,
    partial: &str,
) -> Vec<serenity::AutocompleteChoice> {
    let user = ctx.author();
    let sound_manager = ctx.data().data_manager.sounds();
    let partial = partial.to_lowercase();

    let sounds = sound_manager
        .get_user_sounds(&user.id)
        .await
        .unwrap_or_default();

    sounds
        .iter()
        .filter(|e| e.sound_name.to_lowercase().contains(&partial))
        .map(|e| serenity::AutocompleteChoice::new(e.sound_name.clone(), e.sound_id.to_string()))
        .collect()
}

/// Create an interaction for the search command. Returns the selected video id if any
pub async fn create_public_upload_notice(ctx: Context<'_>) -> Result<Option<bool>, BotError> {
    // Define some unique identifiers for the navigation buttons
    let ctx_id = ctx.id();
    let yes_id = format!("{ctx_id}_yes");
    let no_id = format!("{ctx_id}_no");

    // Send the embed with the first page as content
    let reply = {
        let buttons = vec![
            serenity::CreateButton::new(&yes_id)
                .style(serenity::ButtonStyle::Success)
                .label("Yes"),
            serenity::CreateButton::new(&no_id)
                .style(serenity::ButtonStyle::Danger)
                .label("No"),
        ];

        let description = serenity::MessageBuilder::default()
            .push_line("# Public Upload Notice")
            .push_line("Content uploaded is publicly accessible by default. To upload privately, set the public argument to false.")
            .push_line("")
            .push_bold_line("By clicking Yes, you have read the above and agreed.")
            .push_line("Click *No*, and this reminder will popup again if you try to upload publicly.")
            .push_line("")
            .push_line("This message will not bother you anymore after clicking Yes.")
            .build();

        let embed = serenity::CreateEmbed::default().description(description.to_string());
        let reply = poise::CreateReply::default().embed(embed);

        let components = serenity::CreateActionRow::Buttons(buttons);
        reply.components(vec![components])
    };

    ctx.send(reply).await.context(GeneralSerenitySnafu)?;

    // Loop through incoming interactions with the navigation buttons
    while let Some(press) = serenity::collector::ComponentInteractionCollector::new(ctx)
        // We defined our button IDs to start with `ctx_id`. If they don't, some other command's
        // button was pressed
        .filter(move |press| press.data.custom_id.starts_with(&ctx_id.to_string()))
        // Timeout when no navigation button has been pressed for 1 minute
        .timeout(std::time::Duration::from_secs(60))
        .await
    {
        press
            .create_response(ctx, serenity::CreateInteractionResponse::Acknowledge)
            .await
            .context(GeneralSerenitySnafu)?;

        if press.data.custom_id == yes_id {
            return Ok(Some(true));
        } else if press.data.custom_id == no_id {
            return Ok(Some(false));
        }
    }

    Ok(None)
}

/// Create an interaction for repeating soundboard triggers
pub async fn create_sound_repeat(
    ctx: Context<'_>,
    user_id: &serenity::UserId,
    sound: entity_sqlite::sounds::Model,
    call: Arc<Mutex<songbird::Call>>,
) -> Result<(), BotError> {
    // Define some unique identifiers for the navigation buttons
    let ctx_id = ctx.id();
    let user_id = user_id.get();
    let sound_id = sound.sound_id;
    let repeat_id = format!("{ctx_id}_{user_id}_{sound_id}");
    let mut count = 1;

    let buttons = vec![
        serenity::CreateButton::new(&repeat_id)
            .style(serenity::ButtonStyle::Success)
            .label("Repeat"),
    ];
    let embed = |count: i32| {
        let description = serenity::MessageBuilder::default()
            .push_line(format!("# {}", sound.sound_name))
            .push_line(format!("Played {count} time(s). Play again?"))
            .build();

        embed_template(&EmbedOperation::SoundPlayed).description(description.to_string())
    };
    let components = serenity::CreateActionRow::Buttons(buttons);
    let reply = poise::CreateReply::default()
        .embed(embed(count))
        .components(vec![components]);

    ctx.send(reply).await.context(GeneralSerenitySnafu)?;

    // Loop through incoming interactions with the navigation buttons
    while let Some(press) = serenity::collector::ComponentInteractionCollector::new(ctx)
        // We defined our button IDs to start with `ctx_id`. If they don't, some other command's
        // button was pressed
        .filter(move |press| press.data.custom_id.starts_with(&ctx_id.to_string()))
        // Timeout when no navigation button has been pressed for 1 minute
        .timeout(std::time::Duration::from_secs(300))
        .await
    {
        if press.data.custom_id == repeat_id {
            let path = ctx
                .data()
                .data_dir
                .join("sounds")
                .join(format!("{sound_id}.mp3"));
            tracing::info!("path: {}", path.display());
            let input = songbird::input::File::new(path);

            {
                let mut lock = call.lock().await;
                lock.play(input.into());
            }
            count += 1;
            press
                .create_response(
                    ctx,
                    serenity::CreateInteractionResponse::UpdateMessage(
                        serenity::CreateInteractionResponseMessage::new().embed(embed(count)),
                    ),
                )
                .await
                .context(GeneralSerenitySnafu)?;
        }
    }

    Ok(())
}

pub mod error {

    use snafu::Snafu;

    use crate::error::{ErrorName, UserFriendlyError};

    #[derive(Debug, Snafu)]
    pub enum SoundboardError {
        #[snafu(display("Timeout waiting for input on notice."))]
        NoticeTimeout,

        #[snafu(display("You chose to not allow public uploads."))]
        PolicyDeclined,

        #[snafu(display("File uploaded is not an audio file."))]
        NotAudioFile,
    }

    impl ErrorName for SoundboardError {
        fn name(&self) -> String {
            let str = match self {
                SoundboardError::NoticeTimeout => "notice_timeout",
                SoundboardError::PolicyDeclined => "policy_declined",
                SoundboardError::NotAudioFile => "not_audio_file",
            };
            format!("soundboard::{str}")
        }
    }

    impl UserFriendlyError for SoundboardError {
        fn help_text(&self) -> &str {
            match self {
                SoundboardError::NoticeTimeout => {
                    "Just click the buttons, or you can just ignore me lmao."
                }
                SoundboardError::PolicyDeclined => {
                    "Rerun the command with the public argument set to false."
                }
                SoundboardError::NotAudioFile => "Upload a real audio file instead.",
            }
        }

        fn category(&self) -> crate::error::ErrorCategory {
            crate::error::ErrorCategory::UserMistake
        }
    }
}
