//! Vector Vigil v0.5.x native voice-channel pilot.
//! Creates a persistent voice channel plus a linked discussion channel.
//! This does NOT provide listener-only permissions, audio broadcasting,
//! playback synchronization or a livestream UI.

use std::{collections::HashSet, sync::Arc};
use tokio::sync::Mutex;
use vector_sdk::VectorBot;

const VOICE_NAME: &str = "vctrvoice-live-stage";
const COMMENTS_NAME: &str = "vctrvoice-live-comments";

fn admins_from_env() -> HashSet<String> {
    std::env::var("VCTRVOICE_STAGE_ADMINS")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|s| s.starts_with("npub1") && s.len() == 63)
        .map(str::to_owned)
        .collect()
}

#[tokio::main]
async fn main() -> vector_sdk::Result<()> {
    let admins = Arc::new(admins_from_env());
    let community_id = std::env::var("VCTRVOICE_COMMUNITY_ID").unwrap_or_default();
    let nsec = std::env::var("VECTOR_NSEC").unwrap_or_default();

    if admins.is_empty()
        || community_id.len() != 64
        || !community_id.bytes().all(|b| b.is_ascii_hexdigit())
        || !nsec.starts_with("nsec1")
    {
        eprintln!(
            "Not started: configure a separate bot VECTOR_NSEC, VCTRVOICE_COMMUNITY_ID (64 hex), and VCTRVOICE_STAGE_ADMINS (npub list)"
        );
        return Ok(());
    }

    let bot = VectorBot::builder().nsec(nsec).build().await?;
    let serial = Arc::new(Mutex::new(()));

    bot.command("stage", "Prepare a Vector-native voice stage and its comment channel")
        .run(move |ctx| {
            let admins = Arc::clone(&admins);
            let community_id = community_id.clone();
            let serial = Arc::clone(&serial);
            async move {
                // Never disclose a Community identifier or provision channels in a public message.
                if ctx.msg.is_group || ctx.msg.is_mine() || !admins.contains(&ctx.msg.chat_id) {
                    return;
                }
                // Serialize in-process invocations to avoid creating duplicate channels.
                let _guard = serial.lock().await;
                let community = ctx.bot.community(community_id.clone());
                let channels = community.channels().await;

                let voice_id = if let Some(existing) = channels.iter().find(|c| c.name() == VOICE_NAME) {
                    existing.id().to_owned()
                } else {
                    match community.create_channel(VOICE_NAME).await {
                        Ok(id) => id,
                        Err(err) => {
                            let _ = ctx.reply(format!("Could not create voice channel: {err}")).await;
                            return;
                        }
                    }
                };

                // The SDK creates a text channel by default. The Core API gives it a voice view.
                if let Err(err) = ctx.bot.core().edit_channel_details(
                    &community_id,
                    &voice_id,
                    VOICE_NAME,
                    Some("Native Vector voice stage; broadcaster/listener permissions must be configured"),
                    "voice",
                ).await {
                    let _ = ctx.reply(format!(
                        "Could not configure channel as voice: {err}. Check Manage Channels permission."
                    )).await;
                    return;
                }

                let comments_id = if let Some(existing) = channels.iter().find(|c| c.name() == COMMENTS_NAME) {
                    existing.id().to_owned()
                } else {
                    match community.create_channel(COMMENTS_NAME).await {
                        Ok(id) => id,
                        Err(err) => {
                            let _ = ctx.reply(format!("Voice channel exists, but comments channel creation failed: {err}")).await;
                            return;
                        }
                    }
                };

                let _ = ctx.reply(format!(
                    "Vector-native stage channels are ready in the Community.\n\nVoice: {VOICE_NAME} ({voice_id})\nComments: {COMMENTS_NAME} ({comments_id})\n\nThis is a channel-provisioning pilot, NOT a live broadcast. Configure moderator roles and listener restrictions in Vector before inviting an audience."
                )).await;
            }
        });

    println!("vctrvoice native-stage bot online: {}", bot.npub());
    bot.on_message(|_bot, _message| async move {}).await?;
    Ok(())
}
