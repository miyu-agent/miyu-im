// 明细（施工 S-1 下，设计 03 第一节第 6 条）：新的在前，一笔一行（日子、谁、平台、原币种金额、换成默认币种的或「未换算」、备注）；
// 上面一排搜号或名字、平台、从哪天到哪天，「只看未换算」；点一行开改一笔的浮窗。
import { h, say, money, day, dayStart, platformName, bareId, reasonOf } from './ui.js';

/** 停手多久再搜。 */
const WAIT_MS = 250;

/** `ui` 是 app.js 交的。交回 `{el, load(), onlyPending(), platforms()}`。 */
export function ledgerView(ui) {
  const state = { base: 'CNY', entries: [], known: new Set(['qq']), pendingOnly: false, timer: 0 };
  const query = h('input', { class: 'search', type: 'search', placeholder: say('search'), oninput: () => later() });
  const platform = h('select', { 'aria-label': say('platform'), onchange: () => load() });
  const from = h('input', { type: 'date', title: say('from'), onchange: () => load() });
  const to = h('input', { type: 'date', title: say('to'), onchange: () => load() });
  const pending = h('button', { class: 'chip', type: 'button', text: say('only_pending'), onclick: () => { state.pendingOnly = !state.pendingOnly; paint(); } });
  const filters = h('div', { class: 'filters' }, query, platform,
    h('label', { class: 'date' }, h('span', { text: say('from') }), from),
    h('label', { class: 'date' }, h('span', { text: say('to') }), to), pending);
  const count = h('span', { class: 'count' });
  const rows = h('div', { class: 'rows' });
  const el = h('section', { class: 'panel ledger' },
    h('div', { class: 'section-head' }, h('h2', { text: say('entries') }), count), filters, rows);

  function later() {
    clearTimeout(state.timer);
    state.timer = setTimeout(() => load(), WAIT_MS);
  }

  async function load() {
    const params = {};
    if (query.value.trim()) params.query = query.value.trim();
    if (platform.value) params.platform = platform.value;
    const start = dayStart(from.value);
    const end = dayStart(to.value, true);
    if (start != null) params.from = start;
    if (end != null) params.to = end;
    let got;
    try {
      got = await ui.call('list', params);
    } catch (error) {
      rows.replaceChildren(h('p', { class: 'empty error', text: reasonOf(error) }));
      return;
    }
    state.base = got.base ?? state.base;
    state.entries = got.entries ?? [];
    for (const entry of state.entries) state.known.add(entry.platform);
    platforms();
    paint();
  }

  function platforms() {
    const current = platform.value;
    platform.replaceChildren(h('option', { value: '', text: say('all_platforms') }),
      ...[...state.known].sort().map((one) => h('option', { value: one, text: platformName(one), selected: one === current })));
    platform.hidden = state.known.size < 2;
  }

  /** 这一笔是不是没换算成现在的默认币种。 */
  const unconverted = (entry) => entry.converted == null || entry.base !== state.base;

  function paint() {
    pending.classList.toggle('on', state.pendingOnly);
    const shown = state.pendingOnly ? state.entries.filter(unconverted) : state.entries;
    count.textContent = say('count', { count: shown.length });
    if (shown.length === 0) {
      rows.replaceChildren(h('p', { class: 'empty', text: say(state.entries.length ? 'none_found' : 'none') }));
      return;
    }
    rows.replaceChildren(...shown.map(row));
  }

  function row(entry) {
    const who = entry.name || bareId(entry.user);
    const meta = [platformName(entry.platform), entry.user && entry.name ? bareId(entry.user) : null, entry.user ? null : say('no_id')]
      .filter(Boolean).join(' · ');
    const same = entry.currency === state.base;
    const converted = unconverted(entry)
      ? h('span', { class: 'tag warn', text: say('unconverted') })
      : same ? null : h('span', { class: 'converted', text: `≈ ${money(entry.converted)} ${state.base}` });
    return h('button', { class: 'row', type: 'button', onclick: () => ui.open(entry, state.base) },
      h('span', { class: 'date', text: day(entry.at) }),
      h('span', { class: 'who' }, h('span', { class: 'name', text: who }), h('span', { class: 'meta', text: meta })),
      h('span', { class: 'note', text: entry.note ?? '' }),
      h('span', { class: 'amount' }, h('strong', { text: `${money(entry.amount)} ${entry.currency}` }), converted));
  }

  return {
    el,
    load,
    onlyPending() {
      state.pendingOnly = true;
      paint();
      el.scrollIntoView({ behavior: 'smooth', block: 'start' });
    },
    platforms: () => [...state.known].sort(),
  };
}
