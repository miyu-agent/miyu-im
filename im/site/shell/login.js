// 登录（施工 I-5 上、下，I-6 上从 app.js 挪出来）：交接票换登录（POST /login/ticket）、用户名密码登录（POST /login/password），
// 交回的会话号放进 localStorage（只有这个端口的页面读得到；cookie 同一台机器别的端口也收得到，不用）；登出（POST /logout）。
import { element, say } from './i18n.js';

const KEY = 'miyu-im-session';

/** 记着的会话号；没有的是空的。 */
export function stored() {
  try {
    return localStorage.getItem(KEY);
  } catch {
    return null;
  }
}

/** 记下会话号 `session`，空的是扔掉。 */
export function store(session) {
  try {
    if (session) localStorage.setItem(KEY, session);
    else localStorage.removeItem(KEY);
  } catch {
    // 存不了（隐私模式这类）：这一页照样能用，换一页再进。
  }
}

/** 换登录：`path` 是 /login/ticket 或 /login/password，正文 `body`。成了的记下会话号，交回 {session, account}；不成的交回 {line}。 */
export async function signIn(path, body) {
  let answer;
  try {
    const response = await fetch(path, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });
    answer = { status: response.status, body: await response.json().catch(() => ({})) };
  } catch {
    return { line: say('failed/network') };
  }
  const reason = answer.body.reason;
  if (answer.status === 200) {
    store(answer.body.session);
    return { session: answer.body.session, account: answer.body.account };
  }
  if (reason === 'bad_ticket') return { line: say('failed/ticket') };
  if (reason === 'bad_password') return { line: say('failed/password') };
  if (reason === 'login_throttled') {
    const seconds = Math.ceil((answer.body.retry_after_ms ?? 1000) / 1000);
    return { line: say('failed/throttled', { seconds }) };
  }
  return { line: say('failed/core') };
}

/** 登出：告诉宿主作废这份登录，扔掉会话号。 */
export async function signOut() {
  const session = stored();
  store(null);
  if (!session) return;
  try {
    await fetch('/logout', { method: 'POST', headers: { Authorization: `Bearer ${session}` } });
  } catch {
    // 页面服务不在了：登录本来就没了。
  }
}

/** 在 `root` 里画登录页：上面一行 `line`（可以空），`busy` 的只写那一行；登录成了交给 `onSignedIn({session, account})`。 */
export function showLogin(root, line, onSignedIn, busy = false) {
  document.title = say('title');
  const card = element('div', 'login-card');
  card.append(element('h1', null, say('title')));
  if (line) card.append(element('p', 'login-line', line));
  if (!busy) card.append(form(root, onSignedIn));
  root.replaceChildren(card);
  root.hidden = false;
}

function form(root, onSignedIn) {
  const box = element('form', 'login-form');
  const user = field('user', 'text', 'username');
  const password = field('password', 'password', 'current-password');
  const submit = element('button', 'btn primary', say('sign-in'));
  submit.type = 'submit';
  box.append(user.label, password.label, submit, element('p', 'login-hint', say('signed-out')));
  box.addEventListener('submit', async (event) => {
    event.preventDefault();
    showLogin(root, say('signing-in'), onSignedIn, true);
    const done = await signIn('/login/password', { user: user.input.value, password: password.input.value });
    if (done.session) onSignedIn(done);
    else showLogin(root, done.line, onSignedIn);
  });
  return box;
}

function field(key, type, autocomplete) {
  const label = element('label');
  const input = element('input', 'input');
  input.type = type;
  input.name = key;
  input.autocomplete = autocomplete;
  input.required = true;
  label.append(element('span', null, say(key)), input);
  return { label, input };
}
