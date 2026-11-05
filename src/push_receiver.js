// Rail Web Push receiver. Runs in the hidden background WebView of `rail --background`.
// One autopush WebSocket per subscription; decrypts RFC 8291 (aes128gcm) payloads and
// hands them to Rust, which shows the native desktop notification.
(() => {
  const AUTOPUSH = 'wss://push.services.mozilla.com/';
  const post = (o) => window.webkit.messageHandlers.railPushMsg.postMessage(JSON.stringify(o));
  const unb64u = (s) => {
    s = s.replace(/-/g, '+').replace(/_/g, '/');
    while (s.length % 4) s += '=';
    const b = atob(s);
    const u = new Uint8Array(b.length);
    for (let i = 0; i < b.length; i++) u[i] = b.charCodeAt(i);
    return u;
  };
  const concat = (...a) => {
    const out = new Uint8Array(a.reduce((n, x) => n + x.length, 0));
    let o = 0;
    for (const x of a) { out.set(x, o); o += x.length; }
    return out;
  };
  const enc = new TextEncoder();
  const hkdf = async (salt, ikm, info, len) => {
    const k = await crypto.subtle.importKey('raw', ikm, 'HKDF', false, ['deriveBits']);
    return new Uint8Array(await crypto.subtle.deriveBits({ name: 'HKDF', hash: 'SHA-256', salt, info }, k, len * 8));
  };
  async function decrypt(sub, dataB64) {
    const buf = unb64u(dataB64);
    const salt = buf.slice(0, 16);
    const rs = new DataView(buf.buffer, buf.byteOffset + 16, 4).getUint32(0);
    const idlen = buf[20];
    const asPub = buf.slice(21, 21 + idlen);
    const body = buf.slice(21 + idlen);
    const priv = await crypto.subtle.importKey('jwk', sub.privJwk, { name: 'ECDH', namedCurve: 'P-256' }, false, ['deriveBits']);
    const peer = await crypto.subtle.importKey('raw', asPub, { name: 'ECDH', namedCurve: 'P-256' }, false, []);
    const secret = new Uint8Array(await crypto.subtle.deriveBits({ name: 'ECDH', public: peer }, priv, 256));
    const uaPub = unb64u(sub.p256dh);
    const ikm = await hkdf(unb64u(sub.auth), secret, concat(enc.encode('WebPush: info\0'), uaPub, asPub), 32);
    const cek = await hkdf(salt, ikm, enc.encode('Content-Encoding: aes128gcm\0'), 16);
    const nonce = await hkdf(salt, ikm, enc.encode('Content-Encoding: nonce\0'), 12);
    const key = await crypto.subtle.importKey('raw', cek, 'AES-GCM', false, ['decrypt']);
    const parts = [];
    const step = rs > 0 ? rs : body.length;
    for (let off = 0, seq = 0; off < body.length; off += step, seq++) {
      const n = nonce.slice();
      for (let i = 0; i < 4; i++) n[11 - i] ^= Math.floor(seq / Math.pow(256, i)) & 255;
      const pt = new Uint8Array(await crypto.subtle.decrypt({ name: 'AES-GCM', iv: n }, key, body.slice(off, off + step)));
      let end = pt.length;
      while (end > 0 && pt[end - 1] === 0) end--;
      if (end === 0) throw new Error('bad padding');
      parts.push(pt.slice(0, end - 1));
    }
    return new TextDecoder().decode(concat(...parts));
  }

  const conns = new Map();
  function open(c) {
    if (c.dead) return;
    let ws;
    try { ws = new WebSocket(AUTOPUSH); } catch (_) { return later(c); }
    c.ws = ws;
    ws.onopen = () => {
      ws.send(JSON.stringify({ messageType: 'hello', use_webpush: true, uaid: c.sub.uaid, channelIDs: [c.sub.channelID] }));
      clearInterval(c.ping);
      c.ping = setInterval(() => { try { ws.send('{}'); } catch (_) {} }, 120000);
    };
    ws.onmessage = async (e) => {
      let m;
      try { m = JSON.parse(e.data); } catch (_) { return; }
      if (m.messageType === 'hello') {
        c.backoff = 5000;
        if (m.status !== 200 || (m.uaid && m.uaid !== c.sub.uaid)) post({ log: 'hello mismatch ' + c.sub.id + ' ' + m.status });
      } else if (m.messageType === 'notification') {
        let payload = null;
        try { if (m.data) payload = await decrypt(c.sub, m.data); }
        catch (err) { post({ log: 'decrypt failed ' + c.sub.id + ': ' + err }); }
        post({ id: c.sub.id, payload, hasData: !!m.data });
        try { ws.send(JSON.stringify({ messageType: 'ack', updates: [{ channelID: m.channelID, version: m.version, code: 100 }] })); } catch (_) {}
      }
    };
    ws.onclose = () => { clearInterval(c.ping); later(c); };
    ws.onerror = () => {};
  }
  function later(c) {
    if (c.dead) return;
    c.backoff = Math.min((c.backoff || 5000) * 2, 300000);
    clearTimeout(c.timer);
    c.timer = setTimeout(() => open(c), c.backoff);
  }
  window.railPushSync = (subs) => {
    const want = new Map(subs.map((s) => [s.id + '|' + s.channelID, s]));
    for (const [k, c] of conns) {
      if (!want.has(k)) { c.dead = true; clearInterval(c.ping); clearTimeout(c.timer); try { c.ws.close(); } catch (_) {} conns.delete(k); }
    }
    for (const [k, s] of want) {
      if (!conns.has(k)) { const c = { sub: s, backoff: 2500 }; conns.set(k, c); open(c); }
    }
  };
  // Reconnect promptly when the network comes back.
  window.addEventListener('online', () => { for (const c of conns.values()) { if (!c.ws || c.ws.readyState > 1) { clearTimeout(c.timer); open(c); } } });
  post({ log: 'receiver ready' });
})();
