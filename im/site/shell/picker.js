// 选一项的小浮窗（2026-10-20 项目主人验收：模型多了原生下拉找不到）：头上搜索框，下面照组列出来，点一项交回它、关窗。会话浮窗的
// 选模型（列表）、选表情（格子）都用它；页面上放一个像下拉的按钮（pickButton），点了开这个小窗。
//
// - 第一项可以是「继承（…）」（值是 null），搜的时候它照字比。
// - `custom(字)` 交回一个值的（填了表情的数字编号），列表最上面多一行「用这个」，回车选它；不然回车选列出来的第一项。
import { element, icon, say } from './i18n.js';
import { closeButton, openLayer } from './dialog.js';

/**
 * 开选一项的小浮窗。`options`：
 * - title、placeholder：标题、搜索框的占位字。
 * - groups：[{ name?, items: [{ value, text, sub?, glyph? }] }]，`name` 不写的不画组名。
 * - first：{ value: null, text } 放在最前面的一项（继承），可不写。
 * - current：现在选着的值（那一项画成选中）。
 * - grid：真的排成格子（表情），不然一行一项。
 * - custom：(搜的字) → 值或 null；`customText(值)` 是那一行的字。
 * - pick(value)：选了的。
 */
export function openPicker({ title, placeholder, groups, first, current, grid, custom, customText, pick }) {
  const dialog = element('div', 'dialog pk-dialog');
  const head = element('div', 'd-head');
  const search = element('input', 'input pk-search');
  search.placeholder = placeholder || '';
  search.autocomplete = 'off';
  const list = element('div', 'pk-body');
  dialog.append(head, element('div', 'pk-top'), list);
  dialog.children[1].append(search);
  const close = openLayer(dialog);
  head.append(element('h2', null, title), closeButton(close));
  const chosen = (value) => {
    close();
    pick(value);
  };

  // 回车选的那一项（{value}），一项都没列出来的是 null。
  let firstShown = null;
  const draw = () => {
    const query = search.value.trim();
    const matches = (item, group) => fuzzy(query, `${group?.name || ''} ${item.text} ${item.sub || ''} ${item.value ?? ''}`) >= 0;
    const parts = [];
    firstShown = null;
    const typed = custom ? custom(search.value.trim()) : null;
    if (typed != null) {
      parts.push(itemButton({ value: typed, text: customText(typed) }, current, chosen, 'pk-item pk-custom'));
      firstShown = { value: typed };
    }
    if (first && matches(first)) {
      parts.push(itemButton(first, current, chosen, 'pk-item pk-first'));
      firstShown ??= { value: first.value };
    }
    for (const group of groups) {
      // 照像不像排（施工 I-50）：每个词都整个出现的排在前面，字照先后零散出现的排在后面；一样像的照原来的先后。
      const items = group.items
        .map((item, at) => ({ item, at, score: fuzzy(query, `${group.name || ''} ${item.text} ${item.sub || ''} ${item.value ?? ''}`) }))
        .filter((one) => one.score >= 0)
        .sort((a, b) => b.score - a.score || a.at - b.at)
        .map((one) => one.item);
      if (items.length === 0) continue;
      if (group.name) parts.push(element('div', 'pk-group', group.name));
      const box = element('div', grid ? 'pk-grid' : 'pk-list');
      for (const item of items) box.append(itemButton(item, current, chosen, grid ? 'pk-cell' : 'pk-item'));
      parts.push(box);
      firstShown ??= { value: items[0].value };
    }
    if (parts.length === 0) parts.push(element('p', 'faint pk-none', say('picker/none')));
    list.replaceChildren(...parts);
  };
  search.addEventListener('input', draw);
  search.addEventListener('keydown', (event) => {
    if (event.key !== 'Enter' || event.isComposing || event.keyCode === 229) return;
    event.preventDefault();
    if (firstShown) chosen(firstShown.value ?? null);
  });
  draw();
  requestAnimationFrame(() => {
    search.focus();
    list.querySelector('.on')?.scrollIntoView({ block: 'center' });
  });
  return close;
}

/** 一项：选着的画成选中。 */
function itemButton(item, current, chosen, className) {
  const on = (item.value ?? null) === (current ?? null);
  const button = element('button', on ? `${className} on` : className);
  button.type = 'button';
  if (item.glyph) button.append(element('span', 'pk-glyph', item.glyph));
  const text = element('span', 'pk-text', item.text);
  button.append(text);
  if (item.sub) button.append(element('small', null, item.sub));
  button.title = item.sub ? `${item.text} · ${item.sub}` : item.text;
  button.addEventListener('click', () => chosen(item.value ?? null));
  return button;
}

/** 像下拉的按钮：写着 `text`，点了调 `open`。交回按钮，`setText(字)` 换上面的字。 */
export function pickButton(text, open) {
  const button = element('button', 'pk-trigger');
  button.type = 'button';
  const label = element('span', 'pk-label', text);
  button.append(label, icon('i-down'));
  button.title = text;
  button.addEventListener('click', open);
  button.setText = (next) => {
    label.textContent = next;
    button.title = next;
  };
  return button;
}

/**
 * 模糊匹配（施工 I-50，项目主人 10-27：选模型的搜索要模糊）：`query` 照空白拆成几个词，每个词都要在 `text` 里找得到（不分大小写，
 * 先后不论）：整个出现的算 2 分，字照先后零散出现的（「g38f」配「gemini-3.8-flash」）算 1 分；有一个词找不到的是 -1。空的都配。
 */
export function fuzzy(query, text) {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  const hay = text.toLowerCase();
  let score = 0;
  for (const word of words) {
    if (hay.includes(word)) {
      score += 2;
      continue;
    }
    let at = 0;
    for (const char of word) {
      at = hay.indexOf(char, at);
      if (at < 0) return -1;
      at += 1;
    }
    score += 1;
  }
  return score;
}
