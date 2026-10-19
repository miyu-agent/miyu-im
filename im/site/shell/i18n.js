// 页面上的字（施工 I-6 上从 app.js 挪出来）：照网址片段的 lang、浏览器的语言挑一种，没有的照英文。字段写 {名字}。
import { TEXTS } from '../texts.js';

let language = pick(navigator.language);

/** `wanted`（`zh-CN`、`ja` 这类）里有的那一种，没有的是 `en`。 */
export function pick(wanted) {
  const short = String(wanted || '').slice(0, 2).toLowerCase();
  return TEXTS[short] ? short : 'en';
}

/** 换成 `wanted` 那一种。 */
export function useLanguage(wanted) {
  language = pick(wanted);
  document.documentElement.lang = language;
}

/** 编号是 `key` 的那一句，字段照 `fields` 换。 */
export function say(key, fields = {}) {
  const text = TEXTS[language][key] ?? TEXTS.en[key] ?? key;
  return text.replace(/\{(\w+)\}/g, (_, name) => fields[name] ?? '');
}

/** 一个元素：`tag`，带 `className`，字是 `text`。 */
export function element(tag, className, text) {
  const made = document.createElement(tag);
  if (className) made.className = className;
  if (text !== undefined) made.textContent = text;
  return made;
}

/** 一个图标：雪碧图里编号是 `id` 的那一个。 */
export function icon(id) {
  const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
  svg.setAttribute('class', 'icon');
  const use = document.createElementNS('http://www.w3.org/2000/svg', 'use');
  use.setAttribute('href', `#${id}`);
  svg.append(use);
  return svg;
}
