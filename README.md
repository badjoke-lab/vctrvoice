# vctrvoice

**Vector-native live audio and live video — under development.**

## Direction after Vector v0.5.1 (Vigil)

Vector v0.5.0 (November 5, 2026) introduced **built-in voice/video calls, Community voice channels, camera and desktop screen sharing, and Vector Web**. The current app release is **v0.5.1** (November 7).

**We do not need to reimplement basic voice chat.** vctrvoice now focuses on:

1. **Native voice stage pilot:** prepare a Vector voice channel, comments, and a private session-control channel. Manage opening/closing/status.
2. **Audio livestream + live comments:** host/speaker vs listener roles, synchronized comments, moderation and audience UX.
3. **Video livestream + live comments:** camera/screen capture, viewer scaling, comments overlay and recovery.

The first item is a **native channel + session-lifecycle pilot**, not an audio/video livestream.

[Read the complete migration and acceptance criteria](docs/VIGIL_MIGRATION.md).

## Native stage pilot (experimental)

A separate Vector SDK bot prepares a reusable native voice channel, a discussion channel, and a private control channel for an existing Vector Community. Session markers are written to the private channel, and start/end notices are posted to the discussion channel.

- Source: [native-stage-bot/](native-stage-bot/)
- Vector **app** compatibility target: `v0.5.1`
- Vector **Rust SDK** package reports `0.11.0`, but the prototype fetches it from the Vector **v0.5.1 Git tag** because its published crate has an obsolete, yanked Nostr dependency. The Nostr fork is pinned to the commit used by Vector v0.5.1.
- The bot must already be a member of the Community and have **Manage Channels** permission.
- **Do not use a personal Nostr private key:** create a bot-only `nsec`.
- It accepts `/stage`, `/stage-start`, `/stage-status` and `/stage-end` only by DM from explicitly allowlisted npubs. They **provision channels and manage session markers**, but do not transmit audio/video or enforce listener-only permissions.

Build without using any identity:

```bash
cargo test --manifest-path native-stage-bot/Cargo.toml
```

To run the bot against a test Community, provide the following environment variables **outside of Git**:

```bash
VECTOR_NSEC='nsec1...BOT_KEY_NOT_YOUR_KEY' \
VCTRVOICE_COMMUNITY_ID='64_HEX_COMMUNITY_ID' \
VCTRVOICE_STAGE_ADMINS='npub1...admin' \
cargo run --manifest-path native-stage-bot/Cargo.toml
```

Send `/stage` to the bot in a **private DM** as one of the permitted admins. It creates/reuses `vctrvoice-live-stage` (native voice view), `vctrvoice-live-comments` (text) and `vctrvoice-live-control` (private). Then use `/stage-start`, `/stage-status` and `/stage-end` to manage an event marker and post comments announcements. **This has not yet been exercised against a live Community.** An OPEN marker does **not** mean media is streaming; END does **not** disconnect participants. Member roles, transmission permissions, privacy and mobile compatibility still need real-device verification.

[Stage bot commands and safety checklist](native-stage-bot/README.md).

## Existing WebRTC prototype

The previous standalone Node.js / WebRTC browser experiment remains in `server.mjs`, `public/`, `vector-bot/`, and `test/` as historical code. It is **not** the target app transport. It was never verified as a native Vector call.

[Historical setup and test instructions](docs/LEGACY_WEBRTC_PROTOTYPE.md).

## Validation and safety

GitHub Actions checks the legacy standalone prototype, the native-stage Rust build **and its session state machine unit tests**. **Passing CI does not prove that the native stage works inside Vector.** The next milestones are creating the three channels in a test Community, verifying Android + desktop voice and comments participation, and then implementing true broadcast-only permissions and a unified comments UI.

Never commit `nsec`, API keys, TURN credentials or invite secrets. No production livestream service is deployed.

## Upstream

- [Vector v0.5.0 (Vigil) release](https://github.com/VectorPrivacy/Vector/releases/tag/v0.5.0)
- [Vector v0.5.1 release](https://github.com/VectorPrivacy/Vector/releases/tag/v0.5.1)

The repository is public, but a public repo does not itself grant an open-source license.
