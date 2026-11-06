// 能搜、能筛、分页的列表（设计 11 G2，施工 I-166）：会话概况的最近判断、插件面板的长列表、使用限制和统计的排行共用这一份。
// 头上一条：搜索框、几个筛选下拉；下面照它们筛，超过 `page` 条的末尾一个「再显示」。只照看得见的字和行上带的标签筛，不问宿主。
// 样子在 shell/listing.css。
import { element, say } from './i18n.js';

/**
 * `items` 是 [{ node, text, tags? }]（`text` 是搜索照着找的字，`tags` 是 {筛选的键: 这一行的值}）。`options` 是
 * { search?: 占位字, filters?: [{ key, options: [[值, 给人看的]], all?: 「全部」那一项的字 }], page?: 一次显示几条（出厂 20）,
 *   state?: 记着搜了什么、筛了什么、显示到几条的对象（重画时照旧）, empty?: 一条都对不上时的字 }。交回元素。
 */
export function listing(items, options = {}) {
  const state = options.state || {};
  state.query ??= '';
  state.picked ??= {};
  const page = options.page || 20;
  state.shown ??= page;
  const box = element('div', 'listing');
  const body = element('div', 'listing-body');
  const more = element('button', 'btn ghost listing-more');
  more.type = 'button';
  const filters = options.filters || [];
  const head = element('div', 'listing-head');
  if (options.search) {
    const input = element('input', 'input listing-search');
    input.type = 'search';
    input.placeholder = options.search;
    input.value = state.query;
    input.addEventListener('input', () => {
      state.query = input.value;
      state.shown = page;
      fill();
    });
    head.append(input);
  }
  for (const filter of filters) {
    const pick = element('select', 'select');
    if (filter.all) pick.append(new Option(filter.all, ''));
    for (const [value, text] of filter.options) pick.append(new Option(text, value));
    state.picked[filter.key] ??= filter.all ? '' : filter.options[0]?.[0] ?? '';
    pick.value = state.picked[filter.key];
    pick.addEventListener('change', () => {
      state.picked[filter.key] = pick.value;
      state.shown = page;
      fill();
    });
    head.append(pick);
  }
  const fill = () => {
    const query = state.query.trim().toLowerCase();
    const wanted = items.filter((item) =>
      (!query || item.text.toLowerCase().includes(query))
      && filters.every((filter) => !state.picked[filter.key] || item.tags?.[filter.key] === state.picked[filter.key]));
    const shown = wanted.slice(0, state.shown);
    body.replaceChildren(...(shown.length > 0 ? shown.map((item) => item.node) : [element('p', 'faint listing-none', options.empty || say('listing/none'))]));
    const left = wanted.length - shown.length;
    more.hidden = left <= 0;
    more.textContent = say('listing/more', { count: Math.min(left, page) });
  };
  more.addEventListener('click', () => {
    state.shown += page;
    fill();
  });
  if (head.childElementCount > 0) box.append(head);
  box.append(body, more);
  fill();
  return box;
}
