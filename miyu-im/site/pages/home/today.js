// 首页的「今天」（施工 I-100，项目主人 11-02 定：首页留一个总的数，点进去看数据统计）：token、花费（施工 I-105）、收到、回复、
// 缓存命中率，前四个和昨天比；没价钱的请求底下写一行（同数据统计）。跟着首页选中的连接（「全部」是合计）；整块点了去数据统计（#stats），那一页算的是全部。
import { element, icon, say } from '../../shell/i18n.js';
import { goTo } from '../../shell/connection.js';
import { delta, money, short, tile, unpriced } from '../stats/stats.js';

// 同一个连接隔多久才再问（毫秒）。
const AGAIN = 30000;

/** 首页的「今天」一栏：`request` 调宿主的方法。交回 { head, body, update(picked), dispose() }。 */
export function todayPanel(request) {
  let disposed = false;
  let shown = null;
  let asked = 0;
  // 上一次问的连接、什么时候问到的。
  let scope = null;
  let loadedAt = 0;
  const head = element('div', 'section-head');
  const more = element('a', 'today-more');
  more.href = '#stats';
  more.append(document.createTextNode(say('home/to-stats')), icon('i-right'));
  head.append(element('h2', null, say('home/today')), element('span', 'grow'), more);
  const body = element('div', 'home-today');
  body.addEventListener('click', () => goTo('stats'));
  const draw = () => {
    const token = shown?.token;
    const now = shown?.messages?.now;
    const before = shown?.messages?.prev;
    const rate = token?.hit_rate == null ? '—' : `${Math.round(token.hit_rate * 100)}%`;
    const tiles = element('div', 'stat-tiles');
    tiles.append(
      tile(say('stats/tokens'), short(token?.total), token && delta(token.total, token.prev_total)),
      tile(say('stats/cost'), money(token?.cost?.amount, token?.cost?.currency), token && delta(token.cost?.amount, token.prev_cost)),
      tile(say('stats/received'), short(now?.received), now && delta(now.received, before.received)),
      tile(say('stats/replied'), short(now?.replied), now && delta(now.replied, before.replied)),
      tile(say('stats/hit-rate'), rate, null),
    );
    body.replaceChildren(tiles, unpriced(token?.cost || {}));
  };
  const load = async (picked) => {
    const mine = ++asked;
    let answer;
    try {
      answer = await request('stats', picked ? { period: 'today', connection: picked.id } : { period: 'today' });
    } catch {
      return;
    }
    if (disposed || mine !== asked) return;
    shown = answer;
    loadedAt = Date.now();
    draw();
  };
  draw();
  return {
    head,
    body,
    update(picked) {
      const wanted = picked?.id || '';
      if (wanted === scope && Date.now() - loadedAt < AGAIN) return;
      scope = wanted;
      load(picked);
    },
    dispose() {
      disposed = true;
    },
  };
}
