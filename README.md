# vctrvoice

**Vector-native live audio and live video — under development.**

## Direction after Vector v0.5.1 (Vigil)

Vector v0.5.0 (November 5, 2026) introduced **built-in voice/video calls, Community voice channels, camera and desktop screen sharing, and Vector Web**. The current app release is **v0.5.1** (November 7).

**We do not need to reimplement basic voice chat.** vctrvoice now focuses on:

1. **Native voice stage pilot:** create a Vector voice channel with a linked comments channel.
2. **Audio livestream + live comments:** host/speaker vs listener roles, synchronized comments, moderation and audience UX.
3. **Video livestream + live comments:** camera/screen capture, viewer scaling, comments overlay and recovery.

The first item is a pilot **channel setup**, not an actual livestream.

[Read the complete migration and acceptance criteria](docs/VIGIL_MIGRATION.md).

## Native stage pilot (experimental)

A separate Vector SDK bot prepares a reusable native voice channel and discussion channel for an existing Vector Community.

- Source: [native-stage-bot/](native-stage-bot/)
- Vector **app** compatibility target: `v0.5.1`
- Vector **Rust SDK** dependency: `vector_sdk = "=0.11.0"` (different version series)
- The bot must already be a member of the Community and have **Manage Channels** permission.
- **Do not use a personal Nostr private key:** create a bot-only `nsec`.
- It accepts `/stage` only by DM from explicitly allowlisted npubs, and **only provisions channels**.

Build without using any identity:

```bash
cargo check --manifest-path native-stage-bot/Cargo.toml
```

To run the bot against a test Community, provide the following environment variables **outside of Git**:

```bash
VECTOR_NSEC='nsec1...BOT_KEY_NOT_YOUR_KEY' \
VCTRVOICE_COMMUNITY_ID='64_HEX_COMMUNITY_ID' \
VCTRVOICE_STAGE_ADMINS='npub1...admin' \
cargo run --manifest-path native-stage-bot/Cargo.toml
```

Send `/stage` to the bot in a **private DM** as one of the permitted admins. It creates/reuses `vctrvoice-live-stage` (native voice view) and `vctrvoice-live-comments` (text). **This has not yet been run against a live Community.** Member roles, transmission permissions, privacy, and mobile compatibility still need real-device verification.

## Existing WebRTC prototype

The previous standalone Node.js / WebRTC browser experiment remains in `server.mjs`, `public/`, `vector-bot/`, and `test/` as historical code. It is **not** the target app transport. It was never verified as a native Vector call.

[Historical setup and test instructions](docs/LEGACY_WEBRTC_PROTOTYPE.md).

## Validation and safety

GitHub Actions checks the standalone prototype and the native-stage Rust bot build. **Compilation does not prove that the native stage works inside Vector.** The next real milestones are creating the two channels in a test Community, verifying Android + desktop participation, and then implementing true broadcast-only roles and comments UI.

Never commit `nsec`, API keys, TURN credentials or invite secrets. No production livestream service is deployed.

## Upstream

- [Vector v0.5.0 (Vigil) release](https://github.com/VectorPrivacy/Vector/releases/tag/v0.5.0)
- [Vector v0.5.1 release](https://github.com/VectorPrivacy/Vector/releases/tag/v0.5.1)

The repository is public, but a public repo does not itself grant an open-source license.
