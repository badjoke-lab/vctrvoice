#!/usr/bin/env bash
# Experimental Mac–Android browser voice test.
set -euo pipefail
cd "$(dirname "$0")/.."
export PATH="/opt/homebrew/bin:/usr/local/bin:$PATH"
command -v node >/dev/null || { echo "Node.js 22+ required"; exit 1; }
node -e 'process.exit(Number(process.versions.node.split(".")[0])>=22?0:1)' || { echo "Upgrade to Node.js 22+"; exit 1; }
command -v cloudflared >/dev/null || { echo "Install: brew install cloudflared"; exit 1; }
command -v openssl >/dev/null || { echo "openssl required"; exit 1; }
PORT=8787
if curl -fsS --max-time 1 "http://127.0.0.1:8787/health" >/dev/null 2>&1; then
  echo "Port 8787 already in use"; exit 1;
fi
TMP_VOICE="$(mktemp -d /tmp/vctrvoice.XXXXXX)"
chmod 700 "$TMP_VOICE"
TUNNEL_PID='' SERVER_PID=''
cleanup() {
  if [[ -n "$SERVER_PID" ]]; then kill "$SERVER_PID" 2>/dev/null || true; fi
  if [[ -n "$TUNNEL_PID" ]]; then kill "$TUNNEL_PID" 2>/dev/null || true; fi
  rm -rf "$TMP_VOICE"
}
trap cleanup EXIT
cloudflared tunnel --url "http://127.0.0.1:8787" >"$TMP_VOICE/tunnel.log" 2>&1 &
TUNNEL_PID=$!
echo "Starting temporary HTTPS tunnel..."
PUBLIC_URL=''
for ((i=0;i<80;i++)); do
  PUBLIC_URL="$(grep -Eo 'https://[a-zA-Z0-9-]+\.trycloudflare\.com' "$TMP_VOICE/tunnel.log" | head -1 || true)"
  if [[ -n "$PUBLIC_URL" ]]; then break; fi
  if ! kill -0 "$TUNNEL_PID" 2>/dev/null; then cat "$TMP_VOICE/tunnel.log"; exit 1; fi
  sleep 0.5
done
if [[ -z "$PUBLIC_URL" ]]; then cat "$TMP_VOICE/tunnel.log"; exit 1; fi
ADMIN_KEY="$(openssl rand -hex 32)"
HOST=127.0.0.1 PORT="$PORT" PUBLIC_ORIGIN="$PUBLIC_URL" VOICE_CREATE_KEY="$ADMIN_KEY" \
  ICE_SERVERS_JSON='[{"urls":"stun:stun.l.google.com:19302"}]' node server.mjs >"$TMP_VOICE/server.log" 2>&1 &
SERVER_PID=$!
for ((i=0;i<50;i++)); do
  if curl -fsS --max-time 2 "http://127.0.0.1:8787/health" >/dev/null 2>&1; then break; fi
  if ! kill -0 "$SERVER_PID" 2>/dev/null; then cat "$TMP_VOICE/server.log"; exit 1; fi
  sleep 0.2
done
INVITE="$(PORT="$PORT" ADMIN_KEY="$ADMIN_KEY" node --input-type=module -e '
const r = await fetch("http://127.0.0.1:" + process.env.PORT + "/api/rooms", {
  method:"POST", headers:{"x-voice-admin-key":process.env.ADMIN_KEY}
});
if (!r.ok) throw Error("Room creation failed: HTTP " + r.status);
console.log((await r.json()).url);
')"
printf '\nTemporary invite:\n%s\n\n' "$INVITE"
printf 'Open the complete URL on Mac and Android, enable microphones on both and verify audible speech.\n'
printf 'Never share this URL publicly. Closing this terminal stops the tunnel.\n'
if command -v open >/dev/null 2>&1; then open "$INVITE" || true; fi
while kill -0 "$SERVER_PID" 2>/dev/null && kill -0 "$TUNNEL_PID" 2>/dev/null; do sleep 2; done
echo "Call test stopped."
