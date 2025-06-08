use std::io::Read;

use error::SoundboardError;
use poise::serenity_prelude as serenity;

use crate::{error::BotError, utils::GuildInfo, CommandResult, Context};

use super::join;

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
    ctx.defer_ephemeral().await?;
    let sound_manager = ctx.data().data_manager.sounds();
    let guild_id = GuildInfo::guild_id_or_0(ctx);
    let user_id = ctx.author().id;
    // if not specified, the default is true
    let public = public.unwrap_or(true);

    // show the notice if its a public upload without the policy acceptance
    if public
        && !sound_manager
            .get_user_public_upload_policy(&user_id)
            .await?
    {
        let result = create_public_upload_notice(ctx).await?;
        ctx.defer_ephemeral().await?;

        match result {
            Some(true) => {
                sound_manager
                    .set_user_public_upload_policy(&user_id, true)
                    .await?;
                ctx.reply("Okay, we won't bother you with that anymore.")
                    .await?;
            }
            Some(false) => {
                sound_manager
                    .set_user_public_upload_policy(&user_id, false)
                    .await?;
                return Err(SoundboardError::PolicyDeclined.into());
            }
            None => {
                ctx.reply("Timeout. Upload canceled.").await?;
                return Err(SoundboardError::NoticeTimeout.into());
            }
        }
    }

    ctx.defer_ephemeral().await?;

    let sound_dir = ctx.data().data_dir.join("sounds");
    std::fs::create_dir_all(&sound_dir).map_err(|error| BotError::FilesystemAccessError {
        error,
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
            return Err(BotError::DownloadAttachmentError(e));
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
    std::fs::write(&temp_input_path, &downloaded_file).map_err(|error| {
        BotError::FilesystemAccessError {
            error,
            path: temp_input_path.clone(),
        }
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
        .map_err(BotError::ExternalCommandError)?;

    let mut output = Vec::new();
    recv.read_to_end(&mut output)
        .map_err(|error| BotError::OtherError(Box::new(error)))?;
    tracing::debug!("ffmpeg{}", String::from_utf8(output).unwrap_or_default());

    let success = command
        .wait()
        .await
        .expect("error waiting for command")
        .success();

    if !success {
        ctx.reply("Unable to add sound, not an audio file?").await?;
        return Err(SoundboardError::NotAudioFile.into());
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
        .await?;
    tracing::info!(
        "Added sound {description} with id {sound_id}, publicity {public:?} from user {}",
        ctx.author()
    );

    ctx.reply(format!("Added sound {description} with id {sound_id}"))
        .await?;
    Ok(())
}

#[poise::command(slash_command, guild_only, prefix_command, category = "Soundboard")]
pub async fn play_sound(
    ctx: Context<'_>,
    #[autocomplete = "autocomplete_play_sound"]
    #[description = "The sound identifier. Refer to the autocomplete"]
    sound_id: uuid::Uuid,
) -> Result<(), BotError> {
    ctx.defer_ephemeral().await?;
    join::join_inner(ctx, false, true).await?;

    let manager = ctx.data().songbird.clone();
    let guild_id = crate::utils::get_guild_id(ctx)?;
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
    ctx.reply("Sound played.").await?;
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

    ctx.send(reply).await?;

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
            .await?;

        if press.data.custom_id == yes_id {
            return Ok(Some(true));
        } else if press.data.custom_id == no_id {
            return Ok(Some(false));
        }
    }

    Ok(None)
}

pub mod error {
    use miette::Diagnostic;
    use thiserror::Error;

    use crate::error::ErrorName;

    #[derive(Error, Debug, Diagnostic)]
    pub enum SoundboardError {
        #[error("Timeout waiting for input on notice.")]
        #[diagnostic(help("Just click the buttons, or you can just ignore me lmao."))]
        NoticeTimeout,
        #[error("You chose to not allow public uploads.")]
        #[diagnostic(help("Rerun the command with the public argument set to false."))]
        PolicyDeclined,
        #[error("File uploaded is not an audio file.")]
        #[diagnostic(help("Upload a real audio file instead."))]
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
}
