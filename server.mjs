import http from 'node:http';
import { readFile } from 'node:fs/promises';
import { randomBytes, createHash, createHmac, timingSafeEqual } from 'node:crypto';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const base = dirname(fileURLToPath(import.meta.url));
const host = process.env.HOST || '127.0.0.1';
const port = Number(process.env.PORT || 8787);
const publicOrigin = new URL(process.env.PUBLIC_ORIGIN || 'http://localhost:' + port).origin;
const createKey = (process.env.VOICE_CREATE_KEY || '').length >= 32 ? process.env.VOICE_CREATE_KEY : null;
const loopback = ['127.0.0.1', 'localhost', '::1'].includes(host);
const localOrigin = ['127.0.0.1', 'localhost', '[::1]'].includes(new URL(publicOrigin).hostname);
const demo = process.env.VOICE_ALLOW_PUBLIC_CREATE === '1' && !createKey && loopback && localOrigin;
const rooms = new Map();
const ROOM_TTL = 2 * 60 * 60 * 1000;
const JOIN_TTL = 10 * 60 * 1000;
const POLL_TTL = 45 * 1000;
const LIMIT_BYTES = 64 * 1024;

function token(bytes = 24) { return randomBytes(bytes).toString('base64url'); }
function digest(s) { return createHash('sha256').update(s).digest(); }
function same(value, hash) {
  return typeof value === 'string' && timingSafeEqual(digest(value), hash);
}
function send(res, status, object) {
  if (res.headersSent) return;
  res.writeHead(status, {
    'content-type': 'application/json; charset=utf-8',
    'cache-control': 'no-store',
    'referrer-policy': 'no-referrer',
    'x-content-type-options': 'nosniff'
  });
  res.end(JSON.stringify(object));
}
function json(req) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    let size = 0;
    req.on('data', chunk => {
      size += chunk.length;
      if (size > LIMIT_BYTES) {
        reject(new Error('Request body too large'));
        req.destroy();
      } else chunks.push(chunk);
    });
    req.on('end', () => {
      try { resolve(JSON.parse(Buffer.concat(chunks).toString('utf8'))); }
      catch { reject(new Error('Invalid JSON')); }
    });
    req.on('error', reject);
  });
}
function enqueue(peer, type, data = {}) {
  peer.seq += 1;
  peer.events.push({ seq: peer.seq, type, data });
  if (peer.events.length > 256) peer.events.splice(0, peer.events.length - 256);
}
function leave(room, key) {
  if (!room.sessions.delete(key)) return;
  for (const peer of room.sessions.values()) if (peer.ready) {
    enqueue(peer, 'peer-left');
    enqueue(peer, 'role-update', { role: 'caller' });
  }
}
function active(room, key) {
  const p = room.sessions.get(key);
  return p && p.expires > Date.now() ? p : null;
}
function iceConfig(room) {
  let servers = [];
  try {
    const value = JSON.parse(process.env.ICE_SERVERS_JSON || '[]');
    if (Array.isArray(value)) servers = value.filter(s =>
      s && typeof s === 'object' && !('credential' in s) && !('username' in s)
    ).slice(0, 4);
  } catch { /* invalid optional env */ }
  const secret = process.env.TURN_SHARED_SECRET;
  const urls = (process.env.TURN_URIS || '').split(',').map(s => s.trim()).filter(Boolean);
  if (secret && urls.length && urls.every(u => /^turns?:[^\s]+$/i.test(u))) {
    const username = String(Math.floor(room.expires / 1000) + 60) + ':' + token(12);
    const credential = createHmac('sha1', secret).update(username).digest('base64');
    servers.push({ urls, username, credential });
  }
  return servers;
}

const server = http.createServer(async (req, res) => {
  try {
    const path = new URL(req.url, 'http://localhost').pathname;
    if (req.headers.origin && req.headers.origin !== publicOrigin) {
      return send(res, 403, { error: 'Origin denied' });
    }
    if (req.method === 'GET' && ['/', '/client.js'].includes(path)) {
      const js = path === '/client.js';
      const body = await readFile(join(base, 'public', js ? 'client.js' : 'index.html'));
      res.writeHead(200, {
        'content-type': js ? 'text/javascript; charset=utf-8' : 'text/html; charset=utf-8',
        'cache-control': 'no-store',
        'x-content-type-options': 'nosniff',
        'referrer-policy': 'no-referrer',
        'content-security-policy': "default-src 'self'; connect-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; media-src 'self' blob:; object-src 'none'; frame-ancestors 'none'"
      });
      return res.end(body);
    }
    if (req.method === 'GET' && path === '/health')
      return send(res, 200, { ok: true, version: '0.5.0', signaling: 'json-poll' });
    if (req.method === 'GET' && path === '/api/config')
      return send(res, 200, { publicCreate: demo });
    if (req.method === 'POST' && path === '/api/rooms') {
      if (!demo && (!createKey || !same(req.headers['x-voice-admin-key'], digest(createKey))))
        return send(res, 403, { error: 'Room creation denied' });
      if (rooms.size >= 100) return send(res, 429, { error: 'Too many rooms' });
      const roomId = token(16), invite = token(32);
      const room = { id: roomId, keyHash: digest(invite), expires: Date.now() + ROOM_TTL, sessions: new Map() };
      rooms.set(roomId, room);
      const url = publicOrigin + '/#room=' + encodeURIComponent(roomId) + '&key=' + encodeURIComponent(invite);
      return send(res, 201, { id: roomId, url, expiresAt: room.expires });
    }
    const m = /^\/api\/rooms\/([a-zA-Z0-9_-]{12,40})\/(join|ice|poll|signal|leave)$/.exec(path);
    if (!m) return send(res, 404, { error: 'Not found' });
    const room = rooms.get(m[1]), action = m[2];
    if (!room || room.expires <= Date.now())
      return send(res, 404, { error: 'Room unavailable' });
    if (req.method !== 'POST') return send(res, 405, { error: 'Method not allowed' });
    const data = await json(req);
    if (action === 'join') {
      if (!same(data?.key, room.keyHash)) return send(res, 403, { error: 'Invalid invite' });
      for (const [key, p] of room.sessions) if (p.expires <= Date.now()) leave(room, key);
      if (room.sessions.size >= 2) return send(res, 409, { error: 'Room full' });
      const key = token(32);
      room.sessions.set(key, { expires: Date.now() + JOIN_TTL, ready: false, events: [], seq: 0, lastPoll: 0 });
      return send(res, 201, { session: key });
    }
    const peer = active(room, data?.session);
    if (!peer) return send(res, 403, { error: 'Not joined' });
    if (action === 'ice') return send(res, 200, { iceServers: iceConfig(room) });
    if (action === 'leave') { leave(room, data.session); return send(res, 200, { ok: true }); }
    if (action === 'poll') {
      const cursor = data.cursor;
      if (!Number.isSafeInteger(cursor) || cursor < 0 || cursor > peer.seq)
        return send(res, 400, { error: 'Invalid cursor' });
      peer.lastPoll = Date.now();
      if (!peer.ready) {
        const count = [...room.sessions.values()].filter(p => p.ready).length;
        peer.ready = true;
        peer.expires = room.expires;
        enqueue(peer, 'init', { role: count === 0 ? 'caller' : 'callee' });
        if (count === 1) for (const p of room.sessions.values()) if (p.ready) enqueue(p, 'peer-ready');
      }
      peer.events = peer.events.filter(event => event.seq > cursor);
      return send(res, 200, { events: peer.events });
    }
    if (action === 'signal') {
      if (!peer.ready) return send(res, 403, { error: 'Not ready' });
      if (!['offer', 'answer', 'ice'].includes(data.type) || !data.data ||
          typeof data.data !== 'object' || Array.isArray(data.data))
        return send(res, 400, { error: 'Invalid signal' });
      for (const [key, p] of room.sessions) {
        if (key !== data.session && p.ready) enqueue(p, 'signal', { type: data.type, data: data.data });
      }
      return send(res, 200, { ok: true });
    }
    return send(res, 405, { error: 'Method not allowed' });
  } catch (err) {
    if (!res.headersSent && !res.destroyed)
      send(res, 400, { error: err instanceof Error ? err.message : 'Bad request' });
  }
});

setInterval(() => {
  const now = Date.now();
  for (const [id, room] of rooms) {
    if (room.expires <= now) { rooms.delete(id); continue; }
    for (const [key, peer] of room.sessions) {
      if (peer.expires <= now || (peer.ready && peer.lastPoll && now - peer.lastPoll > POLL_TTL))
        leave(room, key);
    }
  }
}, 10000).unref();

server.listen(port, host, () => console.log('vctrvoice signaling at ' + publicOrigin));
