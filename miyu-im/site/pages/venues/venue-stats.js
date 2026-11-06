// 会话浮窗「概况」里这个会话的统计（施工 I-101，项目主人 11-02 定「小号统计页」）：今天、7 天、30 天；收到、回复、只记录、token、
// 花费（施工 I-105，没价钱的请求底下写一行）、缓存命中率，和前一段同样长的比；7 天、30 天的再画每天的消息、每天的 token。只算这张卡管的场所：先问 cards.overview 带
// `venues: true` 拿到场所，再问 stats 带 `venues`（陌生群、陌生私聊是名单以外的那些并起来）。
import { element, say } from '../../shell/i18n.js';
import { bars, delta, legend, money, short, tile, unpriced } from '../stats/stats.js';

const PERIODS = ['today', '7d', '30d'];
const REMEMBER = 'miyu-im-venue-stats-period';
// 宿主推了变了，隔多久才再问（毫秒）：一来消息就推，统计要问核心的用量账本。
const AGAIN = 30000;

/** 一张卡的统计：`request` 调宿主的方法，`card()` 交现在这张卡（cardOf 的写法）。交回 { box, refresh() }：refresh 是宿主推了变了，
 *  隔够 `AGAIN` 才再问。 */
export function venueStats(request, card) {
  let period = remembered();
  let shown = null;
  let asked = 0;
  let loadedAt = 0;
  // 一张卡片（设计 11 G1，施工 I-166，样子照 shell/sect.css），名字和切今天、7 天、30 天在第一行。
  const box = element('section', 'sect venue-stats');
  const seg = element('div', 'seg');
  const head = element('div', 'venue-stats-head');
  head.append(element('div', 'sect-name', say('venue/stats')), seg);
  const body = element('div');
  box.append(head, body);
  const drawSeg = () => {
    seg.replaceChildren(
      ...PERIODS.map((one) => {
        const button = element('button', one === period ? 'on' : null, say(`stats/period-${one}`));
        button.addEventListener('click', () => {
          period = one;
          try {
            localStorage.setItem(REMEMBER, one);
          } catch {
            // 记不住就算了。
          }
          drawSeg();
          load();
        });
        return button;
      }),
    );
  };
  const draw = () => {
    const token = shown?.token;
    const now = shown?.messages?.now;
    const before = shown?.messages?.prev;
    const rate = token?.hit_rate == null ? '—' : `${Math.round(token.hit_rate * 100)}%`;
    const tiles = element('div', 'stat-tiles');
    tiles.append(
      tile(say('stats/received'), short(now?.received), now && delta(now.received, before.received)),
      tile(say('stats/replied'), short(now?.replied), now && delta(now.replied, before.replied)),
      tile(say('stats/recorded'), short(now?.recorded), now && delta(now.recorded, before.recorded)),
      tile(say('stats/tokens'), short(token?.total), token && delta(token.total, token.prev_total)),
      tile(say('stats/cost'), money(token?.cost?.amount, token?.cost?.currency), token && delta(token.cost?.amount, token.prev_cost)),
      tile(say('stats/hit-rate'), rate, null),
    );
    const parts = [tiles, unpriced(token?.cost || {})];
    const days = shown?.messages?.days || [];
    if (days.length > 1) {
      const daily = bars(
        say('stats/daily-messages'),
        days.map((day) => [
          day.day.slice(5),
          [day.replied, Math.max(0, day.received - day.replied)],
          `${day.day.slice(5)} · ${say('stats/received')} ${day.received} · ${say('stats/replied')} ${day.replied}`,
        ]),
        ['p0', 'p1'],
      );
      daily.append(legend(['stats/replied', 'stats/received'], ['p0', 'p1']));
      parts.push(daily);
      // 用量账本只列有用量的那几天：没有的天补零，和消息的天对齐。
      const used = new Map((token?.days || []).map((day) => [day.day, day.parts.reduce((sum, one) => sum + one, 0)]));
      parts.push(bars(say('stats/daily-tokens'), days.map((day) => [day.day.slice(5), [used.get(day.day) || 0]]), ['p0']));
    }
    body.replaceChildren(...parts);
  };
  const load = async () => {
    const mine = ++asked;
    let answer;
    try {
      const { venues } = await request('cards.overview', { card: card(), venues: true });
      answer = await request('stats', { period, venues: venues || [] });
    } catch {
      return;
    }
    if (mine !== asked) return;
    shown = answer;
    loadedAt = Date.now();
    draw();
  };
  drawSeg();
  draw();
  load();
  return {
    box,
    refresh() {
      if (Date.now() - loadedAt >= AGAIN) load();
    },
  };
}

/** 记着的那一段；没记过、记不住的是今天。 */
function remembered() {
  try {
    const saved = localStorage.getItem(REMEMBER);
    if (PERIODS.includes(saved)) return saved;
  } catch {
    // 记不住就算了。
  }
  return 'today';
}
