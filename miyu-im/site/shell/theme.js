// 主题（设计 01 第九节第 4 条，施工 I-6 上）：
//
// - 从 WebUI 跳过来的：网址片段带 scheme（light、dark）和 colors（网页软件 design.js 的 page 那一组，下划线的名字）。照它定
//   深浅、把认得的几个颜色写成 CSS 变量（下划线换连字符），记进 sessionStorage（这个标签页刷新了照旧）。这时不给手动切：颜色
//   是网页软件那一套的。
// - 单独打开的：跟随系统，能手动切成浅色、深色（记进 localStorage）。

/** 认得的颜色名（网页软件 design.js 的 page 那一组里页面用到的）。 */
const KNOWN = [
  'surface', 'sidebar_bg', 'surface_1', 'surface_2', 'surface_3', 'text', 'text_soft', 'line', 'line_strong',
  'accent', 'accent_soft', 'on_primary', 'on_primary_container', 'gold', 'ribbon', 'online', 'danger',
  'bubble_shadow', 'card_shadow', 'float_shadow', 'peek_shadow', 'sidebar_active', 'sidebar_active_shadow', 'scrim',
];

const HANDED = 'miyu-im-handed-theme';
const CHOSEN = 'miyu-im-scheme';

/** 照网址片段交来的 `given`（没有的照这个标签页记着的）定主题。交回是不是网页软件交来的（是的不给手动切）。 */
export function applyHanded(given) {
  let theme = given && (given.scheme || given.colors) ? { scheme: given.scheme, colors: given.colors } : null;
  if (theme) remember(sessionStorage, HANDED, JSON.stringify(theme));
  else theme = parse(recall(sessionStorage, HANDED));
  if (!theme) {
    applyChosen(recall(localStorage, CHOSEN) || 'system');
    return false;
  }
  if (theme.scheme === 'light' || theme.scheme === 'dark') document.documentElement.dataset.theme = theme.scheme;
  for (const [name, value] of Object.entries(theme.colors || {})) {
    if (KNOWN.includes(name) && typeof value === 'string' && value.length < 200 && !/[;{}]/.test(value)) {
      document.documentElement.style.setProperty(`--${name.replace(/_/g, '-')}`, value);
    }
  }
  return true;
}

/** 手动切的那一档：system、light、dark。 */
export function chosen() {
  return recall(localStorage, CHOSEN) || 'system';
}

/** 换成 `scheme`（system、light、dark），记下。 */
export function choose(scheme) {
  remember(localStorage, CHOSEN, scheme);
  applyChosen(scheme);
}

function applyChosen(scheme) {
  if (scheme === 'light' || scheme === 'dark') document.documentElement.dataset.theme = scheme;
  else delete document.documentElement.dataset.theme;
}

function parse(text) {
  try {
    return text ? JSON.parse(text) : null;
  } catch {
    return null;
  }
}

function recall(storage, key) {
  try {
    return storage.getItem(key);
  } catch {
    return null;
  }
}

function remember(storage, key, value) {
  try {
    storage.setItem(key, value);
  } catch {
    // 存不了（隐私模式这类）：这一回照样用，刷新了照默认的。
  }
}
