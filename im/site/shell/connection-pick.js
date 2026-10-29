// 头上切连接的下拉（施工 I-85 中，设计 07 第 3.2 节；从会话页切平台的那一个改来，样子照旧 .pf-pick）：连接不止一个时才画。点开一张能搜的
// 列表，每一项写平台、名字（没问到的写号）、号；断开了的灰着照样能选（看它的卡片、记录）。`every` 的头上多一项「全部」（消息记录有，
// 会话页没有：会话是一个连接一套）。
import { element, icon, say } from './i18n.js';
import { mark, platform } from '../pages/home/platforms.js';
import { EVERY } from './connection.js';

/** 切连接：`connections` 是外壳问到的那一份，`chosen` 是选中的那一个（「全部」是空的），选了调 `onPick(id)`。交回元素。 */
export function connectionPick(connections, chosen, { every, onPick }) {
  const holder = element('div', 'pf-pick');
  const current = element('button', 'pf-current');
  if (chosen) current.append(platformDot(chosen.platform), element('span', null, connectionLabel(chosen)));
  else current.append(element('span', null, every ? say('conn/all') : say('conn/choose')));
  current.append(icon('i-down'));
  const menu = element('div', 'pf-menu');
  menu.hidden = true;
  const search = element('input', 'input');
  search.placeholder = say('conn/search');
  const list = element('div', 'pf-list');
  const pick = (id) => {
    menu.hidden = true;
    onPick(id);
  };
  const draw = () => {
    const typed = search.value.trim().toLowerCase();
    list.replaceChildren();
    if (every && !typed) {
      const all = element('button', chosen ? null : 'on');
      const mark = element('span', 'pf-dot pf-every');
      mark.append(icon('i-link'));
      all.append(mark, element('span', null, say('conn/all')));
      all.addEventListener('click', () => pick(EVERY));
      list.append(all);
    }
    for (const one of connections) {
      const shown = `${platform(one.platform).name()} ${connectionLabel(one)} ${one.bot}`.toLowerCase();
      if (typed && !shown.includes(typed)) continue;
      const button = element('button', one.id === chosen?.id ? 'on' : null);
      const where = { platform: platform(one.platform).name(), bot: one.bot };
      const line = one.connected ? `${where.platform} · ${where.bot}` : say('conn/off', where);
      button.append(platformDot(one.platform), element('span', null, connectionLabel(one)), element('small', null, line));
      button.addEventListener('click', () => pick(one.id));
      list.append(button);
    }
  };
  search.addEventListener('input', draw);
  menu.append(search, list);
  current.addEventListener('click', (event) => {
    event.stopPropagation();
    menu.hidden = !menu.hidden;
    if (!menu.hidden) search.focus();
  });
  // 焦点离开这一块（点了别处、按 Tab 走了）就收起。
  holder.addEventListener('focusout', (event) => {
    if (!holder.contains(event.relatedTarget)) menu.hidden = true;
  });
  draw();
  holder.append(current, menu);
  return holder;
}

/** 一个连接写成的名字：问到了的名字，没有的号。 */
export function connectionLabel(one) {
  return one.name || String(one.bot);
}

/** 平台 `id` 的小圆点（平台的颜色上画标志）。 */
export function platformDot(id) {
  const shown = element('span', `pf-dot pf-${id}`);
  shown.append(mark(platform(id)));
  return shown;
}
