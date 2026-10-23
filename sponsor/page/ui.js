// 几处共用的（施工 S-1 下）：造元素、字、钱和日子的写法、平台的叫法、问一句。
import { TEXTS } from './texts.js';

const said = { table: TEXTS.en };

/** 照界面的语言挑字（`zh-CN` 取头一段）。 */
export function useLanguage(language) {
  const head = String(language ?? '').toLowerCase().split(/[-_]/)[0];
  said.table = TEXTS[head] ?? TEXTS.en;
}

/** 字 `key`，字段 `{名字}` 照 `fields` 换。 */
export function say(key, fields = {}) {
  const template = said.table[key] ?? TEXTS.en[key];
  if (template == null) return key;
  return template.replace(/\{([a-z_]+)\}/g, (match, name) => (name in fields ? String(fields[name]) : match));
}

/** 造一个元素：`text` 是字，`on…` 是事件，别的是属性；孩子是元素或字（字一律当字）。 */
export function h(tag, props = {}, ...children) {
  const el = document.createElement(tag);
  for (const [key, value] of Object.entries(props)) {
    if (value == null || value === false) continue;
    if (key === 'class') el.className = value;
    else if (key === 'text') el.textContent = value;
    else if (key.startsWith('on')) el.addEventListener(key.slice(2), value);
    else el.setAttribute(key, value === true ? '' : String(value));
  }
  el.append(...children.flat().filter((child) => child != null && child !== false));
  return el;
}

/** 金额：千分位、最多两位小数，去掉多余的零。 */
export function money(value) {
  return Number(value ?? 0).toLocaleString(undefined, { maximumFractionDigits: 2 });
}

/** 毫秒的时刻写成本地的日子：`2026-10-20`。 */
export function day(at) {
  const date = new Date(at);
  if (Number.isNaN(date.getTime())) return '';
  const pad = (n) => String(n).padStart(2, '0');
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())}`;
}

/** `2026-10-20` 这样的日子（本地）零点的毫秒；`end` 是真的算到那天结束。空的是 null。 */
export function dayStart(text, end = false) {
  if (!text) return null;
  const at = new Date(`${text}T00:00`).getTime();
  return Number.isNaN(at) ? null : at + (end ? 86400000 : 0);
}

/** 本月一号、今年一月一号零点（本地）的毫秒；全部的是 null。 */
export function since(period) {
  const now = new Date();
  if (period === 'month') return new Date(now.getFullYear(), now.getMonth(), 1).getTime();
  if (period === 'year') return new Date(now.getFullYear(), 0, 1).getTime();
  return null;
}

/** 平台的叫法：认得的写名字，别的写大写的编号。 */
export function platformName(platform) {
  const known = { qq: 'QQ', tg: 'Telegram', wx: say('platform_wechat') };
  return known[platform] ?? String(platform ?? '').toUpperCase();
}

/** 号只写平台后面那一段：`qq:40001` 写 `40001`。 */
export function bareId(user) {
  return String(user ?? '').split(':').slice(1).join(':') || String(user ?? '');
}

/** 包回的原话：网页把包的错包在 `data.message` 里，没有的照外面的。 */
export function reasonOf(error) {
  return error?.data?.message ?? error?.message ?? String(error);
}
