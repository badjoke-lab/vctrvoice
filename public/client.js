const $ = id => document.getElementById(id);
const show = (id, visible) => $(id).classList.toggle('hidden', !visible);
const status = msg => { $('status').textContent = msg; };
const fragment = new URLSearchParams(location.hash.slice(1));
const room = fragment.get('room');
const inviteKey = fragment.get('key');
const invitation = location.href;
let session = null, role = null, media = null, peer = null, aborter = null;
let iceQueue = [], muted = false;

async function api(path, body) {
  const response = await fetch(path, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(body),
    cache: 'no-store'
  });
  const result = await response.json();
  if (!response.ok) throw new Error(result.error || 'HTTP ' + response.status);
  return result;
}
if (room && inviteKey) {
  show('create', false); show('room', true);
  $('invite').textContent = invitation;
  status('参加できます');
} else {
  status('招待リンクをVectorのDMから受け取ってください');
  fetch('/api/config').then(r => r.json()).then(config => {
    if (config.publicCreate) {
      $('createButton').disabled = false;
      status('ローカル実験モード：通話ルームを作成できます');
    }
  }).catch(() => {});
}
$('createButton').onclick = async () => {
  try {
    const r = await api('/api/rooms', {});
    location.href = r.url;
    location.reload();
  } catch (e) { status('作成失敗: ' + e.message); }
};
$('copyButton').onclick = async () => {
  try { await navigator.clipboard.writeText(invitation); status('コピーしました'); }
  catch { status('リンクを手動でコピーしてください'); }
};
async function signal(type, data) {
  return api('/api/rooms/' + encodeURIComponent(room) + '/signal', { session, type, data });
}
function connect(config) {
  peer?.close();
  peer = new RTCPeerConnection(config);
  iceQueue = [];
  for (const track of media.getTracks()) peer.addTrack(track, media);
  peer.onicecandidate = event => {
    if (event.candidate) signal('ice', event.candidate.toJSON()).catch(console.error);
  };
  peer.ontrack = event => {
    const audio = $('remoteAudio');
    audio.srcObject = event.streams[0] || new MediaStream([event.track]);
    audio.play().then(() => show('playButton', false)).catch(() => {
      show('playButton', true);
      status('相手の音声を再生してください');
    });
  };
  peer.onconnectionstatechange = () => {
    if (peer.connectionState === 'connected') status('通話中');
    else if (['disconnected', 'failed'].includes(peer.connectionState))
      status('通信状態: ' + peer.connectionState);
  };
}
async function receiveSignal(message) {
  if (!peer) return;
  if (message.type === 'ice') {
    if (!peer.remoteDescription) iceQueue.push(message.data);
    else await peer.addIceCandidate(message.data);
  } else if (message.type === 'offer') {
    await peer.setRemoteDescription(message.data);
    for (const item of iceQueue.splice(0)) await peer.addIceCandidate(item);
    await peer.setLocalDescription(await peer.createAnswer());
    await signal('answer', peer.localDescription.toJSON());
  } else if (message.type === 'answer') {
    await peer.setRemoteDescription(message.data);
    for (const item of iceQueue.splice(0)) await peer.addIceCandidate(item);
  }
}
async function processEvent(type, data, config) {
  if (type === 'init') { role = data.role; status('相手を待っています'); }
  else if (type === 'peer-ready') {
    try {
      if (!peer || peer.signalingState === 'closed') connect(config);
      if (role === 'caller') {
        await peer.setLocalDescription(await peer.createOffer());
        await signal('offer', peer.localDescription.toJSON());
      }
    } catch (e) { status('発信失敗: ' + e.message); }
  } else if (type === 'signal') {
    try { await receiveSignal(data); }
    catch (e) { status('接続情報エラー: ' + e.message); }
  } else if (type === 'role-update') role = data.role;
  else if (type === 'peer-left') {
    peer?.close();
    $('remoteAudio').srcObject = null;
    status('相手が退出しました。次の参加者を待っています');
  }
}
async function poll(config, controller, joinedSession) {
  let cursor = 0;
  try {
    while (!controller.signal.aborted) {
      const response = await fetch('/api/rooms/' + encodeURIComponent(room) + '/poll', {
        method: 'POST', headers: { 'content-type': 'application/json' },
        body: JSON.stringify({ session: joinedSession, cursor }),
        signal: controller.signal, cache: 'no-store'
      });
      if (!response.ok) throw new Error('Poll HTTP ' + response.status);
      const result = await response.json();
      for (const event of result.events || []) {
        if (controller.signal.aborted) break;
        if (event.seq <= cursor) continue;
        await processEvent(event.type, event.data, config);
        cursor = event.seq;
      }
      if (!controller.signal.aborted)
        await new Promise(resolve => setTimeout(resolve, 400));
    }
  } catch (e) {
    if (!controller.signal.aborted) { leave(); status('切断: ' + e.message); }
  }
}
$('joinButton').onclick = async () => {
  $('joinButton').disabled = true;
  try {
    if (!room || !inviteKey) throw new Error('有効な招待リンクがありません');
    if (!navigator.mediaDevices?.getUserMedia) throw new Error('HTTPSで開いてください');
    media = await navigator.mediaDevices.getUserMedia({
      audio: { echoCancellation: true, noiseSuppression: true }, video: false
    });
    session = (await api('/api/rooms/' + encodeURIComponent(room) + '/join', { key: inviteKey })).session;
    const ice = await api('/api/rooms/' + encodeURIComponent(room) + '/ice', { session });
    const config = { iceServers: ice.iceServers };
    connect(config);
    aborter = new AbortController();
    void poll(config, aborter, session);
    $('muteButton').disabled = false;
    $('leaveButton').disabled = false;
  } catch (e) { leave(); status('参加失敗: ' + e.message); }
};
$('playButton').onclick = async () => {
  try { await $('remoteAudio').play(); show('playButton', false); status('音声を再生中'); }
  catch (e) { status('再生失敗: ' + e.message); }
};
$('muteButton').onclick = () => {
  muted = !muted;
  for (const track of media?.getAudioTracks() || []) track.enabled = !muted;
  $('muteButton').textContent = muted ? 'ミュート解除' : 'ミュート';
};
function leave() {
  const closingSession = session;
  session = null;
  if (closingSession) {
    fetch('/api/rooms/' + encodeURIComponent(room) + '/leave', {
      method: 'POST', headers: { 'content-type': 'application/json' },
      body: JSON.stringify({ session: closingSession }), keepalive: true
    }).catch(() => {});
  }
  aborter?.abort(); aborter = null; peer?.close(); peer = null;
  media?.getTracks().forEach(t => t.stop()); media = null;
  $('remoteAudio').srcObject = null; show('playButton', false);
  $('joinButton').disabled = false;
  $('muteButton').disabled = true; $('leaveButton').disabled = true;
  muted = false; role = null; $('muteButton').textContent = 'ミュート';
  status('退出しました');
}
$('leaveButton').onclick = leave;
window.addEventListener('pagehide', leave);
setInterval(async () => {
  if (!peer || peer.signalingState === 'closed') {
    $('diagnostics').textContent = '接続診断：未接続'; return;
  }
  try {
    let sent = 0, received = 0, lost = 0;
    for (const s of (await peer.getStats()).values()) {
      if ((s.kind || s.mediaType) !== 'audio') continue;
      if (s.type === 'outbound-rtp') sent += s.packetsSent || 0;
      if (s.type === 'inbound-rtp') { received += s.packetsReceived || 0; lost += s.packetsLost || 0; }
    }
    $('diagnostics').textContent =
      '接続: ' + peer.connectionState + ' / 音声送信: ' + sent +
      ' / 受信: ' + received + ' / ロス: ' + lost;
  } catch { $('diagnostics').textContent = '診断情報を取得できません'; }
}, 3000);
