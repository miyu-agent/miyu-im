// 示范对话一对一对地写（设计 09，施工 I-145；照网页软件的 pairs.js，2026-10-08 项目主人：照旧版一对 user/assistant 地加，不让人手写
// 格式）：一对两行，「你」和人格的名字各一个多行框（不写「她」，人格不一定是女性），右边 ✕ 删这一对；下面「＋ 加一对」。
import { element, icon, say } from '../../shell/i18n.js';

/** 一个多行框：内容 `text`、占位 `hint`、起始 `rows` 行。 */
export function area(text, hint, rows) {
  const box = element('textarea', 'area persona-area');
  box.value = text;
  box.placeholder = hint;
  box.rows = rows;
  return box;
}

/** 示范对话：起始 `pairs`（[{user, assistant}]），`them()` 是人格现在的名字，改了一个字、加了、删了调 `changed`。交回
 *  { el, value(), rename(name), mark(n) }。 */
export function pairsEditor(pairs, them, changed) {
  const rows = [];
  const list = element('div', 'persona-pairs');
  const add = (pair) => {
    const user = area(pair.user, say('personas/user-hint'), 1);
    const assistant = area(pair.assistant, say('personas/reply-hint'), 1);
    const who = element('span', 'persona-who', them());
    const drop = element('button', 'icon-button');
    drop.title = say('personas/drop-pair');
    drop.append(icon('i-x'));
    const turn = (label, box, tail) => {
      const line = element('div', 'persona-turn');
      line.append(label, box, tail);
      return line;
    };
    const el = element('div', 'persona-pair');
    el.append(turn(element('span', 'persona-who', say('personas/you')), user, drop), turn(who, assistant, element('span')));
    const row = { user, assistant, who, el };
    for (const box of [user, assistant]) {
      box.addEventListener('input', () => {
        el.classList.remove('bad');
        changed();
      });
    }
    drop.addEventListener('click', () => {
      rows.splice(rows.indexOf(row), 1);
      el.remove();
      changed();
    });
    rows.push(row);
    list.append(el);
    return row;
  };
  for (const pair of pairs) add(pair);
  const more = element('button', 'btn ghost persona-add');
  more.append(icon('i-plus'), element('span', null, say('personas/add-pair')));
  more.addEventListener('click', () => {
    add({ user: '', assistant: '' }).user.focus();
    changed();
  });
  const el = element('div');
  el.append(list, more);
  return {
    el,
    value: () => rows.map((row) => ({ user: row.user.value, assistant: row.assistant.value })),
    rename(name) {
      for (const row of rows) row.who.textContent = name;
    },
    mark(n) {
      const row = rows[n];
      if (!row) return;
      row.el.classList.add('bad');
      (row.user.value.trim() ? row.assistant : row.user).focus();
    },
  };
}

/** 只写了一头的第一对的序号，没有的是 -1（纯逻辑）。 */
export function halfPair(pairs) {
  return pairs.findIndex((pair) => !pair.user.trim() !== !pair.assistant.trim());
}

/** 去掉两头空白、两头都空的那对拿掉（纯逻辑）。 */
export function cleanPairs(pairs) {
  return pairs
    .map((pair) => ({ user: pair.user.trim(), assistant: pair.assistant.trim() }))
    .filter((pair) => pair.user || pair.assistant);
}
