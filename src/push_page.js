// Rail Web Push shim. WebKitGTK ships no Push API, so sites see "browser not supported".
// This provides PushManager / PushSubscription on top of the Mozilla autopush service
// (a public Web Push endpoint provider). Subscriptions are stored by Rail, and the
// background daemon keeps listening for pushes even when the browser is closed.
(() => {
  if (window.__railPush || !window.isSecureContext) return;
  if (!('serviceWorker' in navigator) || typeof ServiceWorkerRegistration === 'undefined') return;
  if (!window.webkit || !window.webkit.messageHandlers || !window.webkit.messageHandlers.railPush) return;
  const AUTOPUSH = 'wss://push.services.mozilla.com/';
  const b64u = (bytes) => {
    let s = '';
    const u = new Uint8Array(bytes);
    for (let i = 0; i < u.length; i++) s += String.fromCharCode(u[i]);
    return btoa(s).replace(/\+/g, '-').replace(/\//g, '_').replace(/=+$/, '');
  };
  const unb64u = (s) => {
    s = s.replace(/-/g, '+').replace(/_/g, '/');
    while (s.length % 4) s += '=';
    const b = atob(s);
    const u = new Uint8Array(b.length);
    for (let i = 0; i < b.length; i++) u[i] = b.charCodeAt(i);
    return u;
  };
  const call = async (o) => {
    const r = await window.webkit.messageHandlers.railPush.postMessage(JSON.stringify(o));
    return typeof r === 'string' ? JSON.parse(r) : r;
  };
  const toBuf = (u) => u.buffer.slice(u.byteOffset, u.byteOffset + u.byteLength);
  const keyBytes = (k) => {
    if (k == null) return null;
    if (typeof k === 'string') return unb64u(k);
    if (ArrayBuffer.isView(k)) return new Uint8Array(k.buffer, k.byteOffset, k.byteLength);
    return new Uint8Array(k);
  };
  const permissionState = () => {
    const p = Notification.permission;
    return p === 'default' ? 'prompt' : p;
  };
  const subId = async (scope) => {
    const d = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(location.origin + '|' + scope));
    return Array.from(new Uint8Array(d)).slice(0, 16).map((x) => x.toString(16).padStart(2, '0')).join('');
  };

  // One-shot autopush session: hello -> register -> pushEndpoint.
  const registerChannel = (channelID, appKey) => new Promise((resolve, reject) => {
    const ws = new WebSocket(AUTOPUSH);
    let uaid = null;
    const timer = setTimeout(() => { try { ws.close(); } catch (_) {} reject(new DOMException('Push service timed out', 'AbortError')); }, 20000);
    const fail = (m) => { clearTimeout(timer); try { ws.close(); } catch (_) {} reject(new DOMException(m, 'AbortError')); };
    ws.onerror = () => fail('Push service unreachable');
    ws.onclose = () => fail('Push service closed the connection');
    ws.onopen = () => ws.send(JSON.stringify({ messageType: 'hello', use_webpush: true, channelIDs: [] }));
    ws.onmessage = (e) => {
      let m;
      try { m = JSON.parse(e.data); } catch (_) { return; }
      if (m.messageType === 'hello') {
        if (m.status !== 200) return fail('Push service refused the connection');
        uaid = m.uaid;
        const reg = { messageType: 'register', channelID };
        if (appKey) reg.key = b64u(appKey);
        ws.send(JSON.stringify(reg));
      } else if (m.messageType === 'register') {
        if (m.status !== 200 || !m.pushEndpoint) return fail('Push registration failed (' + m.status + ')');
        clearTimeout(timer);
        ws.onclose = null;
        ws.close();
        resolve({ uaid, endpoint: m.pushEndpoint });
      }
    };
  });

  class PushSubscriptionOptions {
    constructor(userVisibleOnly, key) {
      Object.defineProperty(this, 'userVisibleOnly', { value: !!userVisibleOnly, enumerable: true });
      Object.defineProperty(this, 'applicationServerKey', { value: key ? toBuf(key) : null, enumerable: true });
    }
  }
  const SUBS = new WeakMap();
  class PushSubscription {
    constructor(rec, scope) {
      SUBS.set(this, { rec, scope });
      Object.defineProperty(this, 'endpoint', { value: rec.endpoint, enumerable: true });
      Object.defineProperty(this, 'expirationTime', { value: null, enumerable: true });
      Object.defineProperty(this, 'options', {
        value: new PushSubscriptionOptions(rec.userVisibleOnly, rec.appKey ? unb64u(rec.appKey) : null),
        enumerable: true,
      });
    }
    getKey(name) {
      const { rec } = SUBS.get(this);
      if (name === 'p256dh') return toBuf(unb64u(rec.p256dh));
      if (name === 'auth') return toBuf(unb64u(rec.auth));
      return null;
    }
    toJSON() {
      const { rec } = SUBS.get(this);
      return { endpoint: rec.endpoint, expirationTime: null, keys: { p256dh: rec.p256dh, auth: rec.auth } };
    }
    async unsubscribe() {
      const { rec } = SUBS.get(this);
      const r = await call({ op: 'delete', id: rec.id, origin: location.origin });
      return !!(r && r.ok);
    }
  }

  const MANAGERS = new WeakMap();
  class PushManager {
    constructor(token, reg) {
      if (token !== MANAGERS) throw new TypeError('Illegal constructor');
      MANAGERS.set(this, reg);
    }
    static get supportedContentEncodings() { return ['aes128gcm']; }
    async permissionState() { return permissionState(); }
    async getSubscription() {
      const reg = MANAGERS.get(this);
      const id = await subId(reg.scope);
      const r = await call({ op: 'get', id, origin: location.origin });
      return r && r.ok && r.pub ? new PushSubscription(r.pub, reg.scope) : null;
    }
    async subscribe(opts) {
      const reg = MANAGERS.get(this);
      opts = opts || {};
      const appKey = keyBytes(opts.applicationServerKey);
      if (!opts.userVisibleOnly && !appKey) {
        throw new DOMException('Subscribe requires userVisibleOnly or an applicationServerKey', 'NotSupportedError');
      }
      if (Notification.permission !== 'granted') {
        throw new DOMException('Registration failed - permission denied', 'NotAllowedError');
      }
      const existing = await this.getSubscription();
      if (existing) return existing;
      const id = await subId(reg.scope);
      const pair = await crypto.subtle.generateKey({ name: 'ECDH', namedCurve: 'P-256' }, true, ['deriveBits']);
      const p256dh = b64u(await crypto.subtle.exportKey('raw', pair.publicKey));
      const privJwk = await crypto.subtle.exportKey('jwk', pair.privateKey);
      const auth = b64u(crypto.getRandomValues(new Uint8Array(16)));
      const channelID = crypto.randomUUID();
      const { uaid, endpoint } = await registerChannel(channelID, appKey);
      const pub = {
        id, endpoint, p256dh, auth, userVisibleOnly: !!opts.userVisibleOnly,
        appKey: appKey ? b64u(appKey) : null,
      };
      const full = Object.assign({}, pub, {
        origin: location.origin, scope: reg.scope, uaid, channelID, privJwk, created: Date.now(),
      });
      const r = await call({ op: 'save', id, origin: location.origin, pub: JSON.stringify(pub), full: JSON.stringify(full) });
      if (!r || !r.ok) throw new DOMException('Registration failed - ' + ((r && r.error) || 'storage error'), 'AbortError');
      return new PushSubscription(pub, reg.scope);
    }
  }
  const mgrCache = new WeakMap();
  Object.defineProperty(ServiceWorkerRegistration.prototype, 'pushManager', {
    configurable: true,
    enumerable: true,
    get() {
      let m = mgrCache.get(this);
      if (!m) { m = new PushManager(MANAGERS, this); mgrCache.set(this, m); }
      return m;
    },
  });
  for (const [n, c] of [['PushManager', PushManager], ['PushSubscription', PushSubscription], ['PushSubscriptionOptions', PushSubscriptionOptions]]) {
    Object.defineProperty(window, n, { value: c, configurable: true, writable: true });
  }
  Object.defineProperty(window, '__railPush', { value: true });
})();
