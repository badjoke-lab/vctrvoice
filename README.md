# vctrvoice

Voice chat prototype for [Vector](https://vectorapp.io) users, with a planned progression: **1:1 voice → group voice → audio livestream + comments → video livestream + comments**.

## Status

- Experimental WebRTC browser-to-browser audio and a Node.js signaling API.
- Vector SDK invite bot is a **draft**; no live Vector integration has been verified.
- Signaling and authentication can be tested automatically.
- **Not yet verified:** audible Mac↔Android calls, WebRTC interoperability across networks, Vector bot build, end-to-end Nostr identity verification, or livestreaming.
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
