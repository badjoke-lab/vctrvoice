# Native Vector stage controller

This experimental bot is for **Vector v0.5.1** Communities. It uses the SDK package declared as 0.11.0 in Vector's v0.5.1 source tree. This is not a native media producer, a replacement for Vector's voice engine, or a public livestream product.

## Commands (send privately to the bot)

| Command | Effect |
| --- | --- |
| `/stage` | Idempotently creates/reuses a native voice channel, a comments channel and a **private bot-only control channel** |
| `/stage-start` | Records a new session OPEN marker on the control channel, updates the native voice topic, and posts an opening notice in comments |
| `/stage-status` | Reconstructs the session state from the bot-authored control-channel history |
| `/stage-end` | Records a CLOSE marker, updates the native voice topic, and announces the end in comments |

Duplicate `/stage-start` calls when a session is open do not emit a second start marker. Only allowed administrators can invoke these commands; commands posted to Communities do nothing. The bot uses a process-local mutex; **run only one instance per Community** because it is not a distributed lock.

A successful `/stage-start` means **session bookkeeping is open**, not that somebody has joined a call, transmitted audio, or that observers lack microphone access. Likewise, `/stage-end` **does not remove people from the native voice channel**.

Comments are ordinary Vector Community text messages. There is **no integrated player or in-video comments overlay** yet.

## Requirements

1. One test Community in Vector v0.5.1, with the bot already joined.
2. Give the separate bot identity the permissions necessary to create/edit channels.
3. A distinct bot-only Nostr secret key, stored in the environment (not in GitHub).
4. One or more administrator `npub` identifiers.
5. Cargo/Rust and access to Vector's tagged source dependencies.

Example:

```bash
cargo test --manifest-path native-stage-bot/Cargo.toml

# Supply actual secrets through a local secret manager or environment.
VECTOR_NSEC='<BOT_ONLY_NSEC>' \
VCTRVOICE_COMMUNITY_ID='<64_HEX_COMMUNITY_ID>' \
VCTRVOICE_STAGE_ADMINS='<ADMIN_NPUB_1>,<ADMIN_NPUB_2>' \
cargo run --manifest-path native-stage-bot/Cargo.toml
```

The three channel names are `vctrvoice-live-stage` (voice view), `vctrvoice-live-comments` (text), `vctrvoice-live-control` (private). The control channel uses bot-authored messages in the format `VCTRVOICE_STAGE_V1 OPEN|CLOSE <32-hex-session-id>`. Session state survives bot restarts **subject to relay persistence, sync success and channel accessibility**. The control channel's own privacy should be checked in the Vector UI. State recovery is fail-closed on sync errors.

## Required device verification before public use

- Open a session and join the native voice channel from Mac and Android with two distinct user identities.
- Confirm only authorized admin DMs can operate the bot and repeated starts do not open new sessions.
- Restart the bot and ensure `/stage-status` reports the same session.
- Check that text announcements and audience comments appear in the comments channel on both devices.
- Verify audience mic transmission restrictions **in Vector's real permission implementation**. Not demonstrated here.
- Verify that `/stage-end` does not terminate ongoing native calls, and make that visible in the UI.

See [migration plan](../docs/VIGIL_MIGRATION.md).
