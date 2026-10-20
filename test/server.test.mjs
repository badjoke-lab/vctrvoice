import test from 'node:test';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { setTimeout as delay } from 'node:timers/promises';

test('room security and 2-peer signaling', { timeout: 20000 }, async t => {
  const port = 30000 + Math.floor(Math.random() * 20000);
  const base = 'http://127.0.0.1:' + port;
  const admin = 'long-random-admin-key-used-for-tests-only-000000000000';
  const child = spawn(process.execPath, ['server.mjs'], {
    cwd: new URL('../', import.meta.url).pathname,
    env: { ...process.env, PORT: String(port), HOST: '127.0.0.1',
      PUBLIC_ORIGIN: base, VOICE_CREATE_KEY: admin, VOICE_ALLOW_PUBLIC_CREATE: '0' },
    stdio: 'ignore'
  });
  t.after(() => child.kill('SIGTERM'));
  let ready = false;
  for (let i = 0; i < 80; i++) {
    try { if ((await fetch(base + '/health')).ok) { ready = true; break; } }
    catch { await delay(60); }
  }
  assert.equal(ready, true, 'server should start');

  const post = async (path, body, headers = {}) => {
    const r = await fetch(base + path, {
      method: 'POST', headers: { 'content-type': 'application/json', ...headers },
      body: JSON.stringify(body || {})
    });
    return { status: r.status, data: await r.json() };
  };
  assert.equal((await post('/api/rooms', {})).status, 403);
  assert.equal((await post('/api/rooms', {}, { origin: 'https://evil.example',
    'x-voice-admin-key': admin })).status, 403);
  const created = await post('/api/rooms', {}, { 'x-voice-admin-key': admin });
  assert.equal(created.status, 201);
  const { id, url } = created.data;
  const key = new URLSearchParams(new URL(url).hash.slice(1)).get('key');
  assert.ok(key?.length > 30);
  assert.equal(new URL(url).search, '', 'invite secret must not enter URL query');
  const path = '/api/rooms/' + id;
  assert.equal((await post(path + '/join', { key: 'wrong' })).status, 403);
  const first = await post(path + '/join', { key });
  const second = await post(path + '/join', { key });
  assert.equal(first.status, 201);
  assert.equal(second.status, 201);
  const a = first.data.session, b = second.data.session;
  assert.notEqual(a, b);
  assert.equal((await post(path + '/join', { key })).status, 409);
  assert.equal((await post(path + '/ice', { session: 'wrong' })).status, 403);
  assert.equal((await post(path + '/poll', { session: a, cursor: -1 })).status, 400);
  let reply = await post(path + '/poll', { session: a, cursor: 0 });
  assert.equal(reply.data.events.find(e => e.type === 'init')?.data.role, 'caller');
  const aCursor = reply.data.events.at(-1).seq;
  reply = await post(path + '/poll', { session: b, cursor: 0 });
  assert.equal(reply.data.events.find(e => e.type === 'init')?.data.role, 'callee');
  const bCursor = reply.data.events.at(-1).seq;
  reply = await post(path + '/poll', { session: a, cursor: aCursor });
  assert.ok(reply.data.events.some(e => e.type === 'peer-ready'));
  assert.equal((await post(path + '/signal', {
    session: a, type: 'offer', data: { type: 'offer', sdp: 'test' }
  })).status, 200);
  reply = await post(path + '/poll', { session: b, cursor: bCursor });
  assert.equal(reply.data.events.find(e => e.type === 'signal')?.data.data.sdp, 'test');
  const cursor = reply.data.events.at(-1).seq;
  assert.deepEqual((await post(path + '/poll', { session: b, cursor })).data.events, []);
  assert.equal((await post(path + '/leave', { session: a })).status, 200);
  reply = await post(path + '/poll', { session: b, cursor });
  assert.ok(reply.data.events.some(e => e.type === 'peer-left'));
  assert.equal(reply.data.events.find(e => e.type === 'role-update')?.data.role, 'caller');
  const rejoin = await post(path + '/join', { key });
  assert.equal(rejoin.status, 201);
  assert.equal((await post(path + '/signal', { session: 'wrong', type: 'offer', data: {} })).status, 403);
});
