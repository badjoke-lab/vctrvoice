//! Vector v0.5.1 native stage-session controller.
//! The bot provisions native voice + comments, tracks open/closed sessions,
//! and publishes text announcements. This is NOT a standalone livestream,
//! broadcast-only permission system, or proof any media has been transmitted.

mod stage_state;

use std::{collections::HashSet, sync::Arc};
use tokio::sync::Mutex;
use uuid::Uuid;
use vector_sdk::VectorBot;
use stage_state::StageStatus;

const VOICE_NAME: &str = "vctrvoice-live-stage";
const COMMENTS_NAME: &str = "vctrvoice-live-comments";
const CONTROL_NAME: &str = "vctrvoice-live-control";

struct Settings {
    admins: HashSet<String>,
    community_id: String,
    serial: Mutex<()>,
}

struct Channels {
    voice: String,
    comments: String,
    control: String,
}

fn admins_from_env() -> HashSet<String> {
    std::env::var("VCTRVOICE_STAGE_ADMINS")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|s| s.starts_with("npub1") && s.len() == 63)
        .map(str::to_owned)
        .collect()
}

fn authorized(ctx: &vector_sdk::CommandCtx, settings: &Settings) -> bool {
    !ctx.msg.is_group && !ctx.msg.is_mine() && settings.admins.contains(&ctx.msg.chat_id)
}

async fn find_channels(bot: &VectorBot, id: &str) -> Result<Channels, String> {
    let channels = bot.community(id).channels().await;
    let find = |name: &str| {
        channels.iter().find(|channel| channel.name() == name)
            .map(|channel| channel.id().to_owned())
            .ok_or_else(|| format!("{name} is missing. Run /stage first."))
    };
    let control = channels.iter()
        .find(|channel| channel.name() == CONTROL_NAME)
        .ok_or_else(|| "Private stage-control channel is missing. Run /stage first.".to_owned())?;
    if !control.is_private() || !control.is_readable() {
        return Err("Stage-control channel is not private/readable. Stop and check permissions.".into());
    }
    Ok(Channels {
        voice: find(VOICE_NAME)?,
        comments: find(COMMENTS_NAME)?,
        control: control.id().to_owned(),
    })
}

async fn prepare(bot: &VectorBot, id: &str) -> Result<Channels, String> {
    let community = bot.community(id);
    let channels = community.channels().await;
    let voice = if let Some(c) = channels.iter().find(|c| c.name() == VOICE_NAME) {
        c.id().to_owned()
    } else {
        community.create_channel(VOICE_NAME).await.map_err(|e| e.to_string())?
    };

    // Native Vector Core knows the voice channel view.
    bot.core()
        .edit_channel_details(
            id,
            &voice,
            VOICE_NAME,
            Some("vctrvoice: Native voice session; a session marker is not proof of audio transmission"),
            "voice",
        )
        .await.map_err(|e| format!("Set voice view: {e}"))?;

    if !channels.iter().any(|c| c.name() == COMMENTS_NAME) {
        community.create_channel(COMMENTS_NAME).await
            .map_err(|e| format!("Create comments: {e}"))?;
    }
    if !channels.iter().any(|c| c.name() == CONTROL_NAME) {
        community.create_private_channel(CONTROL_NAME).await
            .map_err(|e| format!("Create private control: {e}"))?;
    }
    find_channels(bot, id).await
}

async fn latest_status(bot: &VectorBot, control_id: &str) -> Result<StageStatus, String> {
    let channel = bot.channel(control_id);
    // Synchronize from the relays before making a state-changing decision.
    // One bot process per Community is mandatory; this is not a distributed lock.
    channel.sync(200).await.map_err(|e| format!("Control channel sync failed: {e}"))?;
    let history = channel.history(100).await;
    // The bot-only private channel accepts markers from the bot identity only.
    let owned: Vec<&str> = history.iter().filter(|message|
        message.mine || message.npub.as_deref() == Some(bot.npub())
    ).map(|message| message.content.as_str()).collect();
    Ok(stage_state::replay(owned))
}

async fn post_comment(bot: &VectorBot, comment_id: &str, text: &str) -> Result<(), String> {
    bot.channel(comment_id).send(text).await
        .map(|_| ()).map_err(|e| e.to_string())
}

#[tokio::main]
async fn main() -> vector_sdk::Result<()> {
    let admins = admins_from_env();
    let community_id = std::env::var("VCTRVOICE_COMMUNITY_ID").unwrap_or_default();
    let nsec = std::env::var("VECTOR_NSEC").unwrap_or_default();
    if admins.is_empty()
        || community_id.len() != 64
        || !community_id.bytes().all(|b| b.is_ascii_hexdigit())
        || !nsec.starts_with("nsec1")
    {
        eprintln!(
            "Not started: configure BOT-ONLY VECTOR_NSEC, VCTRVOICE_COMMUNITY_ID (64 hex), and VCTRVOICE_STAGE_ADMINS (npub list)"
        );
        return Ok(());
    }

    let bot = VectorBot::builder().nsec(nsec).build().await?;
    let settings = Arc::new(Settings {
        admins, community_id, serial: Mutex::new(()),
    });

    {
        let settings = Arc::clone(&settings);
        bot.command("stage", "Prepare the native Vector voice, comments and control channels")
            .run(move |ctx| {
                let settings = Arc::clone(&settings);
                async move {
                    if !authorized(&ctx, &settings) { return; }
                    let _guard = settings.serial.lock().await;
                    let result = prepare(&ctx.bot, &settings.community_id).await;
                    let message = match result {
                        Ok(ch) => format!(
                            "Stage channels prepared.\nVoice: {VOICE_NAME} ({})\nComments: {COMMENTS_NAME} ({})\nPrivate control: {CONTROL_NAME} ({})\n\nNo broadcast has started. Use /stage-start to open a session.",
                            ch.voice, ch.comments, ch.control
                        ),
                        Err(err) => format!("Stage preparation failed: {err}"),
                    };
                    let _ = ctx.reply(message).await;
                }
            });
    }

    {
        let settings = Arc::clone(&settings);
        bot.command("stage-start", "Mark the native voice stage open and announce it in comments")
            .run(move |ctx| {
                let settings = Arc::clone(&settings);
                async move {
                    if !authorized(&ctx, &settings) { return; }
                    let _guard = settings.serial.lock().await;
                    let ch = match find_channels(&ctx.bot, &settings.community_id).await {
                        Ok(ch) => ch,
                        Err(err) => { let _ = ctx.reply(err).await; return; }
                    };
                    match latest_status(&ctx.bot, &ch.control).await {
                        Ok(StageStatus::Open(id)) => {
                            let _ = ctx.reply(format!("Session already open ({id}). No duplicate announcement was sent.")).await;
                            return;
                        }
                        Err(err) => { let _ = ctx.reply(err).await; return; }
                        Ok(StageStatus::Closed) => {}
                    }

                    let id = Uuid::new_v4().simple().to_string();
                    let marker = stage_state::opened(&id);
                    if let Err(err) = ctx.bot.channel(&ch.control).send(&marker).await {
                        let _ = ctx.reply(format!("Could not record session start: {err}")).await;
                        return;
                    }
                    // State is durable once the control message is sent. Secondary
                    // operations may fail independently and are never described as succeeded.
                    let notice = format!(
                        "🎙️ vctrvoice session opened ({id}). Join {VOICE_NAME} to speak/listen and post messages here. This marker does not prove that audio is being transmitted."
                    );
                    let comment = post_comment(&ctx.bot, &ch.comments, &notice).await;
                    let topic = format!("Session OPEN ({id}) — not proof of audio transmission");
                    let view = ctx.bot.core().edit_channel_details(
                        &settings.community_id, &ch.voice, VOICE_NAME, Some(&topic), "voice"
                    ).await;
                    let message = format!(
                        "Session OPEN ({id}).\nComments announcement: {}\nVoice topic update: {}\n\nNative audio remains controlled by Vector. Audience-only transmission permissions are NOT yet enforced.",
                        if comment.is_ok() { "sent".into() } else { format!("FAILED: {}", comment.unwrap_err()) },
                        if view.is_ok() { "updated".into() } else { format!("FAILED: {}", view.unwrap_err()) }
                    );
                    let _ = ctx.reply(message).await;
                }
            });
    }

    {
        let settings = Arc::clone(&settings);
        bot.command("stage-end", "Close a native stage session and announce its end")
            .run(move |ctx| {
                let settings = Arc::clone(&settings);
                async move {
                    if !authorized(&ctx, &settings) { return; }
                    let _guard = settings.serial.lock().await;
                    let ch = match find_channels(&ctx.bot, &settings.community_id).await {
                        Ok(ch) => ch,
                        Err(err) => { let _ = ctx.reply(err).await; return; }
                    };
                    let id = match latest_status(&ctx.bot, &ch.control).await {
                        Ok(StageStatus::Open(id)) => id,
                        Ok(StageStatus::Closed) => {
                            let _ = ctx.reply("No open session.").await;
                            return;
                        }
                        Err(err) => { let _ = ctx.reply(err).await; return; }
                    };
                    if let Err(err) = ctx.bot.channel(&ch.control)
                        .send(&stage_state::closed(&id)).await {
                        let _ = ctx.reply(format!("Could not record session end: {err}")).await;
                        return;
                    }
                    let comment = post_comment(
                        &ctx.bot, &ch.comments, &format!("⏹️ vctrvoice session closed ({id}).")
                    ).await;
                    let view = ctx.bot.core().edit_channel_details(
                        &settings.community_id, &ch.voice, VOICE_NAME,
                        Some("Session CLOSED — audio participants must leave the native voice channel separately"),
                        "voice",
                    ).await;
                    let message = format!(
                        "Session CLOSED ({id}).\nComments announcement: {}\nVoice topic update: {}\n\nNOTE: This does NOT forcibly disconnect people from native Vector voice.",
                        if comment.is_ok() { "sent".into() } else { format!("FAILED: {}", comment.unwrap_err()) },
                        if view.is_ok() { "updated".into() } else { format!("FAILED: {}", view.unwrap_err()) }
                    );
                    let _ = ctx.reply(message).await;
                }
            });
    }

    {
        let settings = Arc::clone(&settings);
        bot.command("stage-status", "Read the latest verified stage-session marker")
            .run(move |ctx| {
                let settings = Arc::clone(&settings);
                async move {
                    if !authorized(&ctx, &settings) { return; }
                    let _guard = settings.serial.lock().await;
                    let ch = match find_channels(&ctx.bot, &settings.community_id).await {
                        Ok(ch) => ch,
                        Err(err) => { let _ = ctx.reply(err).await; return; }
                    };
                    let message = match latest_status(&ctx.bot, &ch.control).await {
                        Ok(StageStatus::Open(id)) => format!("Session OPEN ({id})\nVoice: {}\nComments: {}\nThis is bookkeeping, NOT a media-stream health check.", ch.voice, ch.comments),
                        Ok(StageStatus::Closed) => format!("Session CLOSED\nVoice: {}\nComments: {}", ch.voice, ch.comments),
                        Err(err) => format!("Could not read session status: {err}"),
                    };
                    let _ = ctx.reply(message).await;
                }
            });
    }

    println!("vctrvoice native stage controller online: {}", bot.npub());
    bot.on_message(|_bot, _message| async move {}).await?;
    Ok(())
}
