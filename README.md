# vctrvoice

Voice chat prototype for [Vector](https://vectorapp.io) users, with a planned progression: **1:1 voice → group voice → audio livestream + comments → video livestream + comments**.

## Status

- Experimental WebRTC browser-to-browser audio and a Node.js signaling API.
- Vector SDK invite bot **compiles in GitHub Actions**, but a live Vector message round trip has not been verified.
- Signaling and authentication can be tested automatically.
- **Not yet verified:** audible Mac↔Android calls, WebRTC interoperability across networks, a real Vector DM invocation, end-to-end Nostr identity verification, or livestreaming.
- This is **not** native Vector audio calling, and transport encryption is not proof of Nostr-identity-bound end-to-end media authentication.

## Local development

Requires Node.js 22+.

```bash
npm test
VOICE_CREATE_KEY="$(openssl rand -hex 32)" PORT=8787 node server.mjs
```

To create a room:

```bash
curl -s -X POST http://127.0.0.1:8787/api/rooms -H "x-voice-admin-key: $VOICE_CREATE_KEY"
```

Use the generated invitation URL on two devices. A production deployment requires HTTPS and reliable STUN/TURN relay capacity; **do not share private invitation links publicly**.

## Safety / deployment

- Room creation is denied unless `VOICE_CREATE_KEY` is configured.
- Invites grant access to the room; they do not cryptographically prove a user's Nostr identity.
- Browser WebRTC can expose network addresses to call peers; do not use this experimental prototype for sensitive communication.
- Never commit `nsec`, API keys, TURN secrets, or generated invitations.
- The service uses in-memory rooms (a restart invalidates them), a two-hour room expiry, and max two participants.

## Architecture

Vector SDK bot (separate Nostr identity) → private invite link → HTTPS Node.js signaling → WebRTC audio between browsers. Vector transports chat/invite messages, **not** audio packets.

## Roadmap

1. Verify real Mac + Android two-way audio and recover from network changes.
2. Compile and validate SDK bot, then integrate Vector DMs and room invitations.
3. Extend room roles and implement group voice using a suitable SFU.
4. Audio livestream + comments.
5. Video livestream + comments.

The source license is not yet selected; no license is implied by a public repository.

## Mac + Android real-device smoke test

Requires **Node.js 22+**, [cloudflared](https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/downloads/), and macOS with an active microphone.

```bash
brew install cloudflared
git clone https://github.com/badjoke-lab/vctrvoice.git
cd vctrvoice
bash scripts/test-call-mac.sh
```

The script creates a disposable HTTPS Quick Tunnel and prints an invitation URL. Open **the entire link** on the Mac and on Android, allow microphone access on both, and confirm (a) WebRTC connection reaches `connected`, (b) audio packet counters increase, and (c) **you can hear speech in both directions**. A signaling test alone is insufficient.

Because this first test uses a STUN server but no TURN deployment, calls across certain NATs and networks can fail even if the code works. The Quick Tunnel URL is temporary, not a production deployment.

## Experimental Vector SDK bot

The Rust source is in `vector-bot/`. To operate it, configure **a separate bot** `VECTOR_NSEC`, `VOICE_CREATE_KEY` matching the server, `VOICE_SERVER`, and an explicit `VECTOR_VOICE_ALLOWED_NPUBS` list. Never use a personal account's nsec. The bot accepts `/voice` only in direct messages from allowed npubs. **Passing `cargo check` is not equivalent to verifying a live message round trip.**

```bash
cargo check --manifest-path vector-bot/Cargo.toml
```
