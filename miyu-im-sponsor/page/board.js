// 头上的合计和排行榜（施工 S-1 下，设计 03 第一节第 6 条）：本月、今年、全部（换成默认币种的），待补汇率的笔数（点它明细只看
// 没换算的）；排行榜一人一行（有号的照号、只有昵称的照名字），可选时间段、平台。
import { h, say, money, since, platformName, bareId, reasonOf } from './ui.js';

/** `ui` 是 app.js 交的。交回 `{el, load(), base()}`。 */
export function boardView(ui) {
  const state = { base: 'CNY', period: 'all', platform: '' };
  const figures = h('div', { class: 'figures' });
  const seg = h('div', { class: 'seg', role: 'tablist' });
  const platform = h('select', { class: 'platform', 'aria-label': say('platform'), onchange: () => { state.platform = platform.value; rank(); } });
  const rows = h('ol', { class: 'ranking' });
  const head = h('div', { class: 'section-head' }, h('h2', { text: say('ranking') }), h('span', { class: 'spacer' }), seg, platform);
  const el = h('section', { class: 'board' }, figures, h('div', { class: 'panel' }, head, rows));

  for (const period of ['month', 'year', 'all']) {
    seg.append(h('button', {
      type: 'button', role: 'tab', 'data-period': period, text: say(`period_${period}`),
      onclick: () => { state.period = period; paintSeg(); rank(); },
    }));
  }
  const paintSeg = () => {
    for (const button of seg.children) button.classList.toggle('on', button.dataset.period === state.period);
  };
  paintSeg();

  const figure = (label, value, extra) => h('div', { class: 'figure' },
    h('span', { class: 'label', text: label }),
    h('strong', { class: 'value' }, money(value), h('small', { text: ` ${state.base}` })),
    extra);

  async function totals() {
    const got = await ui.call('totals');
    state.base = got.base ?? state.base;
    const pending = got.pending > 0
      ? h('button', { class: 'pending', type: 'button', text: say('pending', { count: got.pending }), onclick: () => ui.pending() })
      : null;
    figures.replaceChildren(
      figure(say('period_month'), got.month),
      figure(say('period_year'), got.year),
      figure(say('period_all'), got.all, pending));
  }

  async function rank() {
    const params = {};
    const from = since(state.period);
    if (from != null) params.from = from;
    if (state.platform) params.platform = state.platform;
    let got;
    try {
      got = await ui.call('ranking', params);
    } catch (error) {
      rows.replaceChildren(h('li', { class: 'empty error', text: reasonOf(error) }));
      return;
    }
    const list = got.rows ?? [];
    if (list.length === 0) {
      rows.replaceChildren(h('li', { class: 'empty', text: say('ranking_empty') }));
      return;
    }
    const top = list[0].sum || 1;
    rows.replaceChildren(...list.map((row, index) => h('li', { class: 'rank' },
      h('span', { class: `place p${Math.min(index + 1, 4)}`, text: String(index + 1) }),
      h('span', { class: 'who' },
        h('span', { class: 'name', text: row.name || bareId(row.user) }),
        h('span', { class: 'meta', text: [platformName(row.platform), row.user && row.name ? bareId(row.user) : null, say(row.count === 1 ? 'times_one' : 'times', { count: row.count })].filter(Boolean).join(' · ') })),
      h('span', { class: 'sum' }, money(row.sum), h('small', { text: ` ${got.base}` })),
      h('span', { class: 'bar-track' }, h('span', { class: 'bar-fill', style: `width: ${Math.max(4, (row.sum / top) * 100)}%` })))));
  }

  /** 平台下拉照明细里出现过的。 */
  function platforms() {
    const known = ui.platforms();
    const current = state.platform;
    platform.replaceChildren(h('option', { value: '', text: say('all_platforms') }),
      ...known.map((one) => h('option', { value: one, text: platformName(one), selected: one === current })));
    platform.hidden = known.length < 2;
  }

  return {
    el,
    base: () => state.base,
    async load() {
      await totals();
      platforms();
      await rank();
    },
  };
}
