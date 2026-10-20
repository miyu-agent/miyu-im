// 统计页（施工 I-14，设计 01 第八节第 4 条：看统计主要是看 token，放最上面）：今天、7 天、30 天，三段从上往下：token、消息、运行。
// 数照宿主的 stats（核心的用量账本、宿主自己记的、进程的），每一样和前一段同样长的比；金额算不准不显示（项目主人的规矩）。
//
// - 场所照 cards.get 的群、好友名单写名字，没有的写编号；点了去会话页打开那个场所的浮窗（#venues/<种类>/<号>）。
// - 图都是条：每天的 token 四样叠着画，构成画成一条，按小时 24 根。
import { element, say } from '../../shell/i18n.js';

const PERIODS = ['today', '7d', '30d'];
const PARTS = ['cache_read', 'uncached', 'cache_write', 'output'];
const REMEMBER = 'miyu-im-stats-period';

/** 在 `view` 里起统计页：`request` 调宿主的方法。 */
export function mountStats(view, { request }) {
  let disposed = false;
  let period = remembered();
  let names = new Map();
  const seg = element('div', 'seg');
  const body = element('div', 'stats-body');
  const head = element('div', 'view-head stats-head');
  head.append(element('h1', null, say('stats/title')), seg);
  view.replaceChildren(head, body);
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
  const load = async () => {
    body.replaceChildren(element('p', 'faint', say('stats/loading')));
    let stats;
    try {
      const [answer, cards] = await Promise.all([request('stats', { period }), request('cards.get').catch(() => null)]);
      stats = answer;
      names = nameMap(cards);
    } catch {
      if (!disposed) body.replaceChildren(element('p', 'faint', say('stats/failed')));
      return;
    }
    if (disposed) return;
    body.replaceChildren(tokens(stats.token, names), messages(stats.messages, names), running(stats.running));
  };
  drawSeg();
  load();
  return {
    update() {},
    event() {},
    dispose() {
      disposed = true;
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

/** 场所编号 → 名字（cards.get 的群、好友）。 */
function nameMap(cards) {
  const map = new Map();
  for (const group of cards?.groups || []) if (group.name) map.set(`group:${group.id}`, group.name);
  for (const friend of cards?.friends || []) if (friend.name) map.set(`private:${friend.id}`, friend.name);
  return map;
}

/** 一个场所（`qq:group:777`）：名字和去浮窗的链接。 */
function venueLink(id, names) {
  const [, kind, number] = String(id).split(':');
  const link = element('a', 'stats-venue', names.get(`${kind}:${number}`) || `${say(`stats/kind-${kind}`)} ${number}`);
  link.href = `#venues/${encodeURIComponent(kind)}/${encodeURIComponent(number)}`;
  return link;
}

/** 一个数照人读的写法：1.2k、3.4M（.0 不写）。 */
function short(value) {
  if (value == null) return '—';
  const one = (number, unit) => `${number.toFixed(1).replace(/\.0$/, '')}${unit}`;
  if (value >= 1e6) return one(value / 1e6, 'M');
  if (value >= 1e3) return one(value / 1e3, 'k');
  return String(Math.round(value));
}

/** 和前一段比：↑12%、↓5%、前一段是零的不写。 */
function delta(now, before) {
  if (now == null || before == null || before === 0) return null;
  const change = Math.round(((now - before) / before) * 100);
  return { text: `${change >= 0 ? '↑' : '↓'}${Math.abs(change)}%`, up: change >= 0 };
}

/** 一格数：名字、值、和前一段比。 */
function tile(label, value, compare) {
  const box = element('div', 'stat-tile');
  box.append(element('span', 'faint', label), element('b', null, value));
  if (compare) box.append(element('small', compare.up ? 'up' : 'down', compare.text));
  return box;
}

/** 一段：标题和几块。 */
function section(title, ...parts) {
  const box = element('section', 'stats-section');
  box.append(element('h2', null, title), ...parts);
  return box;
}

/** 一列「名字 · 数」：`rows` 是 [节点或字, 右边的字]。 */
function list(title, rows) {
  const box = element('div', 'stats-list');
  box.append(element('h3', null, title));
  if (rows.length === 0) box.append(element('p', 'faint', say('stats/none')));
  for (const [name, right] of rows) {
    const row = element('div', 'stats-row');
    row.append(typeof name === 'string' ? element('span', null, name) : name, element('span', 'faint', right));
    box.append(row);
  }
  return box;
}

/** 竖条图：`rows` 是 [标签, [每一段的值…], 悬停的字?]，段照 `classes` 上色、从下往上叠。 */
function bars(title, rows, classes) {
  const box = element('div', 'stats-chart');
  box.append(element('h3', null, title));
  const max = Math.max(1, ...rows.map(([, values]) => values.reduce((sum, one) => sum + one, 0)));
  // 条多的（30 天、24 小时）隔几根写一个标签，最后一根总写。
  const every = Math.ceil(rows.length / 12);
  const plot = element('div', 'bars');
  rows.forEach(([label, values, hover], at) => {
    const column = element('div', 'bar');
    column.title = hover || `${label} · ${values.map(short).join(' / ')}`;
    const stack = element('div', 'stack');
    values.forEach((value, index) => {
      const piece = element('i', classes[index]);
      piece.style.height = `${(value / max) * 100}%`;
      stack.append(piece);
    });
    const shown = at % every === 0 || at === rows.length - 1;
    column.append(stack, element('small', null, shown ? label : ''));
    plot.append(column);
  });
  box.append(plot);
  return box;
}

/** 图例。 */
function legend(keys, classes) {
  const box = element('div', 'legend');
  keys.forEach((key, index) => {
    const item = element('span');
    item.append(element('i', classes[index]), document.createTextNode(say(key)));
    box.append(item);
  });
  return box;
}

/** token 那一段。 */
function tokens(token, names) {
  const rate = (value) => (value == null ? '—' : `${Math.round(value * 100)}%`);
  const tiles = element('div', 'stat-tiles');
  tiles.append(
    tile(say('stats/tokens'), short(token.total), delta(token.total, token.prev_total)),
    tile(say('stats/hit-rate'), rate(token.hit_rate), null),
    tile(say('stats/per-reply'), short(token.per_reply), null),
    tile(say('stats/requests'), short(token.requests), delta(token.requests, token.prev_requests)),
  );
  const classes = ['p0', 'p1', 'p2', 'p3'];
  const keys = PARTS.map((part) => `stats/part-${part}`);
  // 只有一天（今天）的不画每天的图，构成那一条就够了。
  const several = (token.days || []).length > 1;
  const days = several
    ? bars(say('stats/daily-tokens'), token.days.map((day) => [day.day.slice(5), day.parts]), classes)
    : document.createDocumentFragment();
  if (several) days.append(legend(keys, classes));
  // 一个 token 都没有的不画构成、按小时（空条像坏了）。
  const total = token.parts.reduce((sum, one) => sum + one, 0);
  const make = total > 0 ? element('div', 'stats-chart') : document.createDocumentFragment();
  const strip = element('div', 'strip');
  token.parts.forEach((value, index) => {
    const piece = element('i', classes[index]);
    piece.style.width = `${(value / total) * 100}%`;
    piece.title = `${say(keys[index])} ${short(value)}`;
    strip.append(piece);
  });
  if (total > 0) make.append(element('h3', null, say('stats/parts')), strip, legend(keys, classes));
  // 会话的回合不带用途（核心的账本里是空的），是回复。
  const purpose = (name) => {
    const key = `stats/purpose-${name || 'turn'}`;
    const said = say(key);
    return said === key ? name : said;
  };
  const grid = element('div', 'stats-grid');
  grid.append(
    list(say('stats/by-purpose'), (token.purposes || []).map((one) => [purpose(one.name), `${short(one.requests)} · ${short(one.tokens)}`])),
    list(say('stats/by-venue'), (token.venues || []).map((one) => [venueLink(one.name, names), short(one.tokens)])),
    list(say('stats/by-model'), (token.models || []).map((one) => [one.name || '—', `${short(one.requests)} · ${short(one.tokens)}`])),
  );
  return section(say('stats/token'), tiles, days, make, grid);
}

/** 消息那一段。 */
function messages(shown, names) {
  const now = shown.now;
  const before = shown.prev;
  const rate = now.received ? `${Math.round((now.replied / now.received) * 100)}%` : '—';
  const tiles = element('div', 'stat-tiles');
  tiles.append(
    tile(say('stats/received'), short(now.received), delta(now.received, before.received)),
    tile(say('stats/replied'), short(now.replied), delta(now.replied, before.replied)),
    tile(say('stats/reply-rate'), rate, null),
    tile(say('stats/undelivered'), short(now.undelivered), delta(now.undelivered, before.undelivered)),
  );
  const several = (shown.days || []).length > 1;
  const daily = several
    ? bars(
        say('stats/daily-messages'),
        // 整根是收到的，深色那截是回了的。
        shown.days.map((day) => [
          day.day.slice(5),
          [day.replied, Math.max(0, day.received - day.replied)],
          `${day.day.slice(5)} · ${say('stats/received')} ${day.received} · ${say('stats/replied')} ${day.replied}`,
        ]),
        ['p0', 'p1'],
      )
    : document.createDocumentFragment();
  if (several) daily.append(legend(['stats/replied', 'stats/received'], ['p0', 'p1']));
  const hours = (shown.hours || []).some((count) => count > 0)
    ? bars(say('stats/hours'), shown.hours.map((count, hour) => [String(hour), [count]]), ['p1'])
    : document.createDocumentFragment();
  const reason = (name) => {
    const key = `stats/reason-${name}`;
    const said = say(key);
    return said === key ? name : said;
  };
  const reasons = Object.entries(shown.reasons || {}).sort((a, b) => b[1] - a[1]);
  const grid = element('div', 'stats-grid');
  grid.append(
    list(say('stats/busiest'), (shown.venues || []).map((one) => [venueLink(one.name, names), short(one.received)])),
    list(say('stats/reasons'), reasons.map(([name, count]) => [reason(name), short(count)])),
  );
  return section(say('stats/messages'), tiles, daily, hours, grid);
}

/** 运行那一段。 */
function running(shown) {
  const uptime = (seconds) => {
    const days = Math.floor(seconds / 86400);
    const hours = Math.floor((seconds % 86400) / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const parts = [];
    if (days) parts.push(say('stats/uptime-days', { n: days }));
    if (hours) parts.push(say('stats/uptime-hours', { n: hours }));
    parts.push(say('stats/uptime-minutes', { n: minutes }));
    return parts.join(' ');
  };
  const tiles = element('div', 'stat-tiles');
  tiles.append(
    tile(say('stats/uptime'), uptime(shown.uptime || 0), null),
    tile(say('stats/memory'), shown.rss == null ? '—' : `${(shown.rss / 1048576).toFixed(0)} MB`, null),
    tile(say('stats/cpu'), shown.cpu == null ? '—' : `${shown.cpu.toFixed(1)}%`, null),
    tile(say('stats/plugins'), `${shown.plugins?.on ?? 0} / ${shown.plugins?.total ?? 0}`, null),
  );
  if (shown.plugins?.failed) tiles.append(tile(say('stats/plugins-failed'), String(shown.plugins.failed), null));
  return section(say('stats/running'), tiles);
}
