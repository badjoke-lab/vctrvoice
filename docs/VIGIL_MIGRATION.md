# Vector Vigil migration — 2026-11-14

## Verified upstream baseline

- **Vector app:** v0.5.1 (released 2026-11-07). v0.5.0 (2026-11-05) introduced DM voice/video calls, voice Community channels, camera and desktop screen sharing, roles/permissions and Vector Web.
- **Vector Rust SDK:** the upstream `v0.5.1` tag still declares SDK package version `0.11.0` (app and SDK versions differ). The published 0.11.0 crate on crates.io requires yanked `nostr-sdk=0.45.1`. This pilot therefore uses the Vector **v0.5.1 git tag** and patches `nostr`, `nostr-sdk`, and `nostr-database` to the same pinned Vector-maintained fork revision as the official v0.5.1 lockfile. Do not arbitrarily bump SDK to 0.5.1.
- SDK v0.11.0's `Community::create_channel()` creates a text channel. The SDK provides `VectorBot::core()`, enabling `VectorCore::edit_channel_details(..., "voice")` to change that channel to a voice view. This has been checked against tagged upstream source, not yet exercised against a running Community.
- SDK `Channel::send` and `on_message` handle Community text messages. They do **not** automatically stream media, create a stage-only experience, or establish live viewing permissions.
- Vector's own media path uses voice relay transport. **Do not send media packets through Nostr chat messages** and do not substitute the old browser WebRTC server for Vector's built-in call stack.

References:
- [Vector v0.5.0 release](https://github.com/VectorPrivacy/Vector/releases/tag/v0.5.0)
- [Vector v0.5.1 release](https://github.com/VectorPrivacy/Vector/releases/tag/v0.5.1)
- [v0.5.1 SDK implementation](https://github.com/VectorPrivacy/Vector/blob/v0.5.1/crates/vector-sdk/src/lib.rs)
- [v0.5.1 Core channel view API](https://github.com/VectorPrivacy/Vector/blob/v0.5.1/crates/vector-core/src/lib.rs)

## Decision

Keep the repository name `vctrvoice` and the original long-term goal **audio livestream + live comments → video livestream + live comments**. Deprioritize building a competing 1:1 WebRTC voice chat or group call application now that Vector ships those natively.

The previous Node.js / WebRTC prototype is retained as a historical experiment and is **not** the product's target architecture.

## Phase A — native Vector stage pilot (this PR)

An independent Vector SDK bot creates (or reuses) three **Community** channels:

1. `vctrvoice-live-stage` — upgraded to the native `voice` channel view using Vector Core.
2. `vctrvoice-live-comments` — a native Community text channel for audience comments and start/end notices.
3. `vctrvoice-live-control` — a **private** bot-readable channel containing bot-authored session OPEN/CLOSE markers.

An allowlisted administrator sends `/stage` to the bot in a **DM**. The bot needs its own `nsec`, must already be a member of the target Community and have **Manage Channels** permission. It must **never** use a user's nsec. Other DM commands: `/stage-start` opens a session, posts an announcement and sets the native voice channel topic; `/stage-status` reads persisted control history (or the in-process cache) and reports the session marker; `/stage-end` closes the session and announces its end. An in-process mutex serializes these operations. **Only one bot process per Community** is supported; this does not provide a distributed lock.

The marker protocol has unit tests for valid sessions, malformed input, mismatched closure, repeated sessions, and unrelated messages. State survives a restart subject to relay persistence and successful sync. `/stage-start` does NOT prove that native media is flowing. `/stage-end` does NOT force anyone to disconnect. No listener-only transmission permission is enforced.

This pilot prepares native channels and records the lifecycle. It does **not** yet implement livestream broadcasting, viewer-only roles, on-video comment overlays, a viewer count, recording, or a cross-platform streaming UI. **Creating channels or an OPEN marker must not be reported as launching a livestream.**

### Pilot acceptance

- [x] The Rust code compiles against Vector v0.5.1-tagged SDK source (package 0.11.0), and the lifecycle unit tests pass in GitHub Actions.
- [ ] An authorized admin's private `/stage` creates the exact voice, comments, and private control channels in a test Community.
- [ ] A second `/stage` invocation reuses the same channels, without creating duplicates.
- [ ] An unauthorized DM or Community message cannot provision channels.
- [ ] Vector v0.5.1 on Android and desktop can both join the native voice channel.
- [ ] Native comments and start/end announcements appear on both clients.
- [ ] Bot restart retains the OPEN/CLOSED state from the control channel and duplicate `/stage-start` does not open a second session.
- [ ] Privacy and role behavior are checked before public use.

## Phase B — audio livestream + comments

Use a **stage model**, not a free-for-all voice room. The host and approved speakers transmit; listeners consume and comment without acquiring speaker privileges. Implement explicit start/end state, scheduled/upcoming status, listener count, moderation (delete comments, mute/remove), roles, and live-comment UI integrated with a native Vector channel. Persist and authenticate event state with Nostr identity and community permissions. **Do not claim broadcaster-only audio permission is available until verified in Vector; if absent, implement/extend it.**

Acceptance: on two phones + desktop, one host can broadcast continuously while multiple listeners cannot accidentally transmit; comments appear in order, with moderation and reconnect behavior tested.

## Phase C — video livestream + comments

Add camera/screen capture while preserving the exact audience/comment/role model. Verify scale, latency, bandwidth budget, TURN/voice-relay capacity and media encryption properties. For large audiences evaluate selective forwarding or a purpose-built broadcast path; native small-group calls alone do not demonstrate scalable livestream capacity.

Acceptance: a viewer can watch continuous audio/video, read/post moderated comments, join/leave without becoming a publisher, and keep playback synchronized sufficiently for live interaction.

## Known gaps and non-goals

- Current SDK lacks a documented public high-level voice-session join/publish API; existing UI/Core integration must be traced before creating new media layers.
- Channel provisioning is not audience access enforcement. Configure roles in Vector manually for the pilot and validate each relevant permission.
- Community voice and chat are separate native views; a single combined player/comment overlay is future work.
- This PR is designed for review, compilation and a controlled Community trial. No production deployment, native mobile build or end-to-end media test is claimed.
