// Experimental Vector SDK 0.11.0 invite bot.
// This identity must be independent from any user's Nostr account.
use std::collections::HashSet;
use std::sync::Arc;
use serde::Deserialize;
use vector_sdk::VectorBot;

#[derive(Deserialize)]
struct CreatedRoom {
    url: String,
}

#[tokio::main]
async fn main() -> vector_sdk::Result<()> {
    let create_key = std::env::var("VOICE_CREATE_KEY").unwrap_or_default();
    let allowed: HashSet<String> = std::env::var("VECTOR_VOICE_ALLOWED_NPUBS")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|value| value.starts_with("npub1") && value.len() >= 58 && value.len() <= 64)
        .map(str::to_owned)
        .collect();

    if create_key.len() < 32 || allowed.is_empty() {
        eprintln!("Disabled: VOICE_CREATE_KEY and VECTOR_VOICE_ALLOWED_NPUBS are required");
        return Ok(());
    }
    let server = std::env::var("VOICE_SERVER")
        .unwrap_or_else(|_| "http://127.0.0.1:8787".to_owned());
    let local = server.starts_with("http://127.0.0.1:")
        || server.starts_with("http://localhost:");
    if !server.starts_with("https://") && !local {
        eprintln!("Disabled: VOICE_SERVER must be HTTPS except localhost");
        return Ok(());
    }
    let nsec = match std::env::var("VECTOR_NSEC") {
        Ok(value) if value.starts_with("nsec1") => value,
        _ => {
            eprintln!("Disabled: configure the bot's own VECTOR_NSEC (not a user key)");
            return Ok(());
        }
    };

    let bot = VectorBot::builder().nsec(nsec).build().await?;
    let allowed = Arc::new(allowed);
    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .expect("create reqwest client");

    bot.command("voice", "Create a private one-to-one voice room")
        .run(move |ctx| {
            let http = http.clone();
            let server = server.clone();
            let create_key = create_key.clone();
            let allowed = Arc::clone(&allowed);
            async move {
                // Do not create or post a secret invite from a public community.
                if ctx.msg.is_group || ctx.msg.is_mine() {
                    return;
                }
                // For direct messages, chat_id is the sender's npub.
                if !allowed.contains(&ctx.msg.chat_id) {
                    return;
                }
                match http.post(format!("{server}/api/rooms"))
                    .header("x-voice-admin-key", &create_key)
                    .send()
                    .await
                {
                    Ok(response) if response.status().is_success() => {
                        match response.json::<CreatedRoom>().await {
                            Ok(room) => {
                                let _ = ctx.reply(format!(
                                    "Private voice-room invitation. Share only with the intended person: {}",
                                    room.url
                                )).await;
                            }
                            Err(_) => {
                                let _ = ctx.reply("Voice service returned an invalid room").await;
                            }
                        }
                    }
                    _ => {
                        let _ = ctx.reply("Voice service is unavailable").await;
                    }
                }
            }
        });

    println!("vctrvoice invite bot online: {}", bot.npub());
    bot.on_message(|_bot, _msg| async move {}).await?;
    Ok(())
}
