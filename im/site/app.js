// 接入通讯平台页面的入口（设计 01 第九节第 3、4 条，施工 I-5 上）。
//
// 1. 网址片段带 `#miyu=<base64url(JSON)>` 的（WebUI、终端跳过来）：先用 history.replaceState 去掉片段（票别留在地址栏），读出
//    `ticket` 换登录（POST /login/ticket），交回的会话号放进 localStorage（只有这个端口的页面读得到；cookie 同一台机器别的端口
//    也收得到，不用）。`lang` 照它挑字。
// 2. 没带票、localStorage 里有会话号的：开通道（/channel）出示，对得上的算登录着；关闭帧 4401 的扔掉会话号。
// 3. 登出：POST /logout 带会话号，扔掉会话号。
import { TEXTS } from './texts.js';

const KEY = 'miyu-im-session';
const app = document.getElementById('app');
let language = pick(navigator.language);

function pick(wanted) {
  const short = String(wanted || '').slice(0, 2).toLowerCase();
  return TEXTS[short] ? short : 'en';
}

function say(key, fields = {}) {
  const text = TEXTS[language][key] ?? TEXTS.en[key] ?? key;
  return text.replace(/\{(\w+)\}/g, (_, name) => fields[name] ?? '');
}

function stored() {
  try {
    return localStorage.getItem(KEY);
  } catch {
    return null;
  }
}

function store(session) {
  try {
    if (session) localStorage.setItem(KEY, session);
    else localStorage.removeItem(KEY);
  } catch {
    // 存不了（隐私模式这类）：这一页照样能用，换一页再进。
  }
}

function show(line, account) {
  document.title = say('title');
  document.documentElement.lang = language;
  const title = document.createElement('h1');
  title.textContent = say('title');
  const status = document.createElement('p');
  status.textContent = line;
  app.replaceChildren(title, status);
  if (account) {
    const out = document.createElement('button');
    out.type = 'button';
    out.textContent = say('sign-out');
    out.addEventListener('click', signOut);
    app.append(out);
  }
}

function handed() {
  const hash = location.hash;
  if (!hash.startsWith('#miyu=')) return null;
  history.replaceState(null, '', location.pathname + location.search);
  try {
    const base64 = hash.slice(6).replace(/-/g, '+').replace(/_/g, '/');
    const bytes = Uint8Array.from(atob(base64), (c) => c.charCodeAt(0));
    return JSON.parse(new TextDecoder().decode(bytes));
  } catch {
    return null;
  }
}

async function redeem(ticket) {
  show(say('signing-in'));
  let answer;
  try {
    const response = await fetch('/login/ticket', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ ticket }),
    });
    answer = { status: response.status, body: await response.json().catch(() => ({})) };
  } catch {
    show(say('failed/network'));
    return;
  }
  if (answer.status === 200) {
    store(answer.body.session);
    show(say('signed-in', { account: answer.body.account }), answer.body.account);
  } else if (answer.body.reason === 'bad_ticket') {
    show(say('failed/ticket'));
  } else {
    show(say('failed/core'));
  }
}

function resume(session) {
  const channel = new WebSocket(`ws://${location.host}/channel`);
  channel.addEventListener('open', () => channel.send(JSON.stringify({ session })));
  channel.addEventListener('message', (event) => {
    const first = JSON.parse(event.data);
    if (first.account) show(say('signed-in', { account: first.account }), first.account);
  }, { once: true });
  channel.addEventListener('close', (event) => {
    if (event.code === 4401) {
      store(null);
      show(say('signed-out'));
    }
  });
  channel.addEventListener('error', () => show(say('failed/network')));
}

async function signOut() {
  const session = stored();
  store(null);
  if (session) {
    try {
      await fetch('/logout', { method: 'POST', headers: { Authorization: `Bearer ${session}` } });
    } catch {
      // 页面服务不在了：登录本来就没了。
    }
  }
  show(say('signed-out'));
}

const given = handed();
if (given?.lang) language = pick(given.lang);
if (given?.scheme === 'dark' || given?.scheme === 'light') {
  document.documentElement.dataset.theme = given.scheme;
}
if (typeof given?.ticket === 'string') {
  redeem(given.ticket);
} else if (stored()) {
  resume(stored());
} else {
  show(say('signed-out'));
}
