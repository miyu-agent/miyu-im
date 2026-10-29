// 选中的连接（施工 I-85 中，设计 07 第二节第 2 条、第 3.2 节）：整个页面只记一份。首页的连接卡片、会话页和消息记录的下拉都改它，
// 别的页跟着它看。
//
// - 写进网址（#venues?c=tg:7890123456）：刷新、收藏不丢；这台浏览器另记一份上次选的，网址里没写的照它。
// - 「全部」是 all；只有一个连接时页面不画切换器，照那一个看（chosenOf）。
// - 选了的连接没了（删了、驱动不挂了），照「全部」。
// - 网址里的换了（点了带 `?c=` 的链接、前进后退）：照网址的选（施工 I-85 下，统计里的场所链接带着它的连接）。

const KEY = 'miyu-im-connection';
const ALL = 'all';
const listeners = new Set();
let current = initial();

/** 网址的 # 拆成页（`venues`）、页后面那一段（`group/777`）和选中的连接（`?c=` 的值，没写的是空的）。 */
export function hashParts(hash = location.hash) {
  const raw = hash.replace(/^#/, '');
  const at = raw.indexOf('?');
  const route = at < 0 ? raw : raw.slice(0, at);
  const query = new URLSearchParams(at < 0 ? '' : raw.slice(at + 1));
  return { route, connection: query.get('c') || '' };
}

/** 页面起来时选中的：网址里写的，没有的照这台浏览器记的，都没有的是「全部」。 */
function initial() {
  const given = hashParts().connection;
  if (given) return given;
  try {
    return localStorage.getItem(KEY) || ALL;
  } catch {
    return ALL;
  }
}

/** 现在选中的连接编号（`<平台>:<号>`），「全部」是 `all`。 */
export function connection() {
  return current;
}

/** 选中 `id`：记进这台浏览器、写进网址（不添历史），叫一声在听的。 */
export function chooseConnection(id) {
  if (id === current) return;
  current = id;
  try {
    localStorage.setItem(KEY, id);
  } catch {
    // 记不住就算了：这一回照样用。
  }
  goTo(hashParts().route, { replace: true });
  for (const listener of listeners) listener(id);
}

/** 去 `#route`，带上选中的连接（「全部」不写）。`replace` 的不添历史。 */
export function goTo(route, { replace = false } = {}) {
  const hash = current === ALL ? route : `${route}?c=${encodeURIComponent(current)}`;
  if (replace) history.replaceState(null, '', `${location.pathname}${location.search}#${hash}`);
  else location.hash = hash;
}

/** 选中的变了叫 `listener(id)`。交回不再听的那一下。 */
export function onConnection(listener) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** 照这一刻的连接 `connections` 认选中的：只有一个的是那一个；选中的不在了的是「全部」（顺手改掉记下的）。交回那一个连接，「全部」
 *  是空的。 */
export function chosenOf(connections) {
  if (connections.length === 1) return connections[0];
  const found = connections.find((one) => one.id === current);
  if (!found && current !== ALL && connections.length > 0) chooseConnection(ALL);
  return found || null;
}

/** 「全部」的编号。 */
export const EVERY = ALL;

window.addEventListener('hashchange', () => {
  const given = hashParts().connection;
  if (given && given !== current) chooseConnection(given);
});
