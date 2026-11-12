// 数据统计的走势（设计 11 第三节第 3 条，施工 I-169）：原来每天的 token、每天的花费、每天的消息三张图，并成一张能切的：token（四样叠着）、
// 花费（有一天有金额才给这一项）、回复（回了的叠在收到的上面）。只有一天（今天）的不画。切到哪一样记在 `state` 里，重画时照旧。
import { element, say } from '../../shell/i18n.js';
import { sect } from '../../shell/sect.js';
import { bars, legend, money } from './stats.js';

/** 四样 token 的颜色，和构成那一条一样。 */
const PARTS = ['p0', 'p1', 'p2', 'p3'];

/** 走势那一张卡：`shown` 是 stats 的回答；画不出的交回空的片段。 */
export function trend(shown, state) {
  const token = shown.token || {};
  const messages = shown.messages?.days || [];
  // 核心的账本只交有用量的那几天：照消息那边每一天都有的日期补齐，没用量的天画空（不然 30 天只剩几根条，看不出走势）。
  const used = new Map((token.days || []).map((day) => [day.day, day]));
  const days = messages.length > 0
    ? messages.map((one) => used.get(one.day) || { day: one.day, parts: [0, 0, 0, 0], cost: 0 })
    : token.days || [];
  if (days.length <= 1 && messages.length <= 1) return document.createDocumentFragment();
  const currency = token.cost?.currency;
  const modes = ['tokens', ...(days.some((day) => day.cost) ? ['cost'] : []), 'replies'];
  if (!modes.includes(state.mode)) state.mode = modes[0];
  const seg = element('div', 'seg');
  const card = sect(say('stats/trend'), null, seg);
  const chart = element('div');
  const draw = () => {
    seg.replaceChildren(
      ...modes.map((one) => {
        const button = element('button', one === state.mode ? 'on' : null, say(`stats/trend-${one}`));
        button.addEventListener('click', () => {
          state.mode = one;
          draw();
        });
        return button;
      }),
    );
    chart.replaceChildren(...drawn(state.mode, { days, messages, currency }));
  };
  card.body.append(chart);
  draw();
  return card.box;
}

/** 一样的图和图例。 */
function drawn(mode, { days, messages, currency }) {
  if (mode === 'cost') {
    return [bars('', days.map((day) => [day.day.slice(5), [day.cost || 0], `${day.day.slice(5)} · ${money(day.cost, currency)}`]), ['p2'])];
  }
  if (mode === 'replies') {
    // 整根是收到的，深色那截是回了的。
    const rows = messages.map((day) => [
      day.day.slice(5),
      [day.replied, Math.max(0, day.received - day.replied)],
      `${day.day.slice(5)} · ${say('stats/received')} ${day.received} · ${say('stats/replied')} ${day.replied}`,
    ]);
    return [bars('', rows, ['p0', 'p1']), legend(['stats/replied', 'stats/received'], ['p0', 'p1'])];
  }
  const keys = ['cache_read', 'uncached', 'cache_write', 'output'].map((part) => `stats/part-${part}`);
  return [bars('', days.map((day) => [day.day.slice(5), day.parts]), PARTS), legend(keys, PARTS)];
}
