// 接入通讯平台页面的入口（设计 01 第八、九节，施工 I-5 上起，I-6 上拆成外壳和各页）：
//
// 1. 网址片段带 `#miyu=<base64url(JSON)>` 的（WebUI、终端跳过来）：先用 history.replaceState 去掉片段（票别留在地址栏），照它的
//    lang 挑字、scheme 和 colors 定主题（shell/theme.js），ticket 换登录（shell/login.js）。
// 2. 有会话号的开通道（shell/channel.js），宿主回了账号就起外壳（shell/shell.js）；宿主推了变了，外壳再问。会话号不认了（4401）
//    回登录页；断了的外壳头上写连不上，通道自己再连。
// 3. 没有会话号的：登录页。
import { useLanguage } from './shell/i18n.js';
import { applyHanded } from './shell/theme.js';
import { showLogin, signIn, signOut, stored, store } from './shell/login.js';
import { openChannel } from './shell/channel.js';
import { mountShell } from './shell/shell.js';

const login = document.getElementById('login');
const app = document.getElementById('app');

/** 网址片段交来的那一份；没有的、读不懂的是空的。读了就去掉片段。 */
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

/** 照会话号 `session` 开通道、起外壳。 */
function enter(session, fromWebUi) {
  let shell = null;
  const channel = openChannel(session, {
    onOpen() {
      login.hidden = true;
      document.body.classList.add('signed-in');
      if (!shell) {
        shell = mountShell(app, {
          request: (method, params) => channel.request(method, params),
          handed: fromWebUi,
          onSignOut: async () => {
            channel.close();
            shell.dispose();
            await signOut();
            leave('');
          },
        });
      }
      shell.refresh();
    },
    onEvent(name) {
      if (name === 'changed') shell?.refresh();
      else shell?.event(name);
    },
    onLost() {
      shell?.offline();
    },
    onExpired() {
      shell?.dispose();
      store(null);
      leave('');
    },
  });
}

/** 回登录页，上面一行写 `line`。 */
function leave(line) {
  app.hidden = true;
  app.replaceChildren();
  document.body.classList.remove('signed-in');
  showLogin(login, line, (done) => enter(done.session, fromWebUi));
}

const given = handed();
useLanguage(given?.lang || navigator.language);
const fromWebUi = applyHanded(given);
if (typeof given?.ticket === 'string') {
  showLogin(login, '', () => {}, true);
  signIn('/login/ticket', { ticket: given.ticket }).then((done) => {
    if (done.session) enter(done.session, fromWebUi);
    else leave(done.line);
  });
} else if (stored()) {
  enter(stored(), fromWebUi);
} else {
  leave('');
}
