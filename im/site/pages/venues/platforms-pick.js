// 会话页头上切平台的下拉（施工 I-7 上修，设计 01 第八节第 3 条，照样板 d-im.html 的 .pf-pick）：连着不止一个平台时才画。点开一张
// 能搜的列表，每一项写平台、几个机器人、几个会话，没连上的灰着点不了。
import { element, icon, say } from '../../shell/i18n.js';
import { mark, platform } from '../home/platforms.js';

/** 切平台：`items` 是 [{ id, bots, venues }]（venues 不知道的是 null），`chosen` 是现在的，选了调 `onPick(id)`。交回元素。 */
export function platformPick(items, chosen, onPick) {
  const holder = element('div', 'pf-pick');
  const current = element('button', 'pf-current');
  current.append(dot(chosen), element('span', null, platform(chosen).name()), icon('i-down'));
  const menu = element('div', 'pf-menu');
  menu.hidden = true;
  const search = element('input', 'input');
  search.placeholder = say('venues/platform-search');
  const list = element('div', 'pf-list');
  const draw = () => {
    const typed = search.value.trim().toLowerCase();
    list.replaceChildren();
    for (const item of items) {
      const shown = platform(item.id);
      if (typed && !shown.name().toLowerCase().includes(typed)) continue;
      const button = element('button', item.id === chosen ? 'on' : null);
      let line = say('venues/platform-off');
      if (item.bots > 0 && item.venues === null) line = say('venues/platform-bots', { bots: item.bots });
      else if (item.bots > 0) line = say('venues/platform-line', { bots: item.bots, venues: item.venues });
      button.append(dot(item.id), element('span', null, shown.name()), element('small', null, line));
      button.disabled = item.bots === 0;
      button.addEventListener('click', () => {
        menu.hidden = true;
        onPick(item.id);
      });
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

/** 平台 `id` 的小圆点（平台的颜色上画标志）。 */
function dot(id) {
  const shown = element('span', `pf-dot pf-${id}`);
  shown.append(mark(platform(id)));
  return shown;
}
