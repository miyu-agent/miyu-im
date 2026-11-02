// 数据统计（施工 I-14 起是一页，I-85 下并进首页，I-100 又拆回侧栏：项目主人 11-02 定首页只留今天的几个数，点进来看明细；设计 01
// 第八节第 4 条：看统计主要是看 token，放最上面）：今天、7 天、30 天，从上往下：token、消息、运行。数照宿主的 stats（核心的用量账本、
// 宿主自己记的、进程的），每一样和前一段同样长的比。金额照核心换算成你设的币种的写，没价钱的请求写「另有几次请求没记录金额」（施工
// I-104，项目主人 11-02 改了原来的「算不准不显示」）。
//
// - 连接不止一个时标题旁一个下拉（施工 I-106，项目主人 11-02：按连接切换整页，取代 I-100 的「按连接」那一段），同消息记录：有
//   「全部」，跟着整个页面选中的连接（shell/connection.js）。「全部」时场所前面画它的平台标志。
// - 场所照首页给的名字写（cards.get 的群、好友），没有的写编号；点了去会话页打开那个场所的浮窗（#venues/<种类>/<号>，带上它的连接）。
// - 图都是条：每天的 token 四样叠着画，构成画成一条，按小时 24 根。列表是带列名的小表（施工 I-106）。
import { element, say } from '../../shell/i18n.js';
import { connectionPick, platformDot } from '../../shell/connection-pick.js';
import { chooseConnection, chosenOf, onConnection } from '../../shell/connection.js';
import { namesOf } from '../home/activity.js';
import { venueParts } from '../records/shared.js';

const PERIODS = ['today', '7d', '30d'];
const PARTS = ['cache_read', 'uncached', 'cache_write', 'output'];
const REMEMBER = 'miyu-im-stats-period';
// 宿主推了变了，隔多久才再问（毫秒）。
const AGAIN = 30000;

/** 在 `view` 里起数据统计：`request` 调宿主的方法。交回 update(connections)、dispose()。 */
export function mountStats(view, { request }) {
  let disposed = false;
  let period = remembered();
  let connections = [];
  let names = new Map();
  let shown = null;
  let asked = 0;
  let loadedAt = 0;
  // 上一次问的连接（「全部」是空的）。
  let scope = null;
  const seg = element('div', 'seg');
  const pickSlot = element('span', 'pf-slot');
  const head = element('div', 'view-head stats-head');
  head.append(element('h1', null, say('nav/stats')), pickSlot, seg);
  const body = element('div', 'stats-body');
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
  const draw = () => {
    const picked = chosenOf(connections);
    pickSlot.replaceChildren();
    if (connections.length > 1) pickSlot.append(connectionPick(connections, picked, { every: true, onPick: chooseConnection }));
    if (!shown) return;
    const context = { names, connections, tagged: !picked && connections.length > 1 };
    body.replaceChildren(tokens(shown.token, context), messages(shown.messages, context), running(shown.running));
  };
  const load = async () => {
    const mine = ++asked;
    const picked = chosenOf(connections);
    scope = picked?.id || '';
    if (!shown) body.replaceChildren(element('p', 'faint', say('stats/loading')));
    let answer;
    try {
      answer = await request('stats', picked ? { period, connection: picked.id } : { period });
    } catch {
      if (!disposed && mine === asked && !shown) body.replaceChildren(element('p', 'faint', say('stats/failed')));
      return;
    }
    // 等的时候又换了时段、连接的：这一份不要了。
    if (disposed || mine !== asked) return;
    shown = answer;
    loadedAt = Date.now();
    draw();
  };
  drawSeg();
  request('cards.get').then(
    (cards) => {
      names = namesOf(cards);
      draw();
    },
    () => {},
  );
  const unlisten = onConnection(() => load());
  return {
    update(now) {
      connections = now;
      const wanted = chosenOf(connections)?.id || '';
      if (wanted === scope && Date.now() - loadedAt < AGAIN) draw();
      else load();
    },
    dispose() {
      disposed = true;
      unlisten();
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

/** 一个场所（`qq:group:777`）：名字和去浮窗的链接（带上它的连接：选中的那一个，「全部」时照平台、号找）；`tagged` 的前面画平台
 *  标志。 */
function venueLink(id, { names, connections, tagged }) {
  const { platform, type, id: number, bot } = venueParts(id);
  const link = element('a', 'stats-venue');
  if (tagged) link.append(platformDot(platform));
  link.append(document.createTextNode(names.get(`${type}:${number}`) || `${say(`stats/kind-${type}`)} ${number}`));
  const owner = connections.find((one) => one.platform === platform && (!bot || String(one.bot) === bot));
  const query = owner ? `?c=${encodeURIComponent(owner.id)}` : '';
  link.href = `#venues/${encodeURIComponent(type)}/${encodeURIComponent(number)}${query}`;
  return link;
}

/** 一个数照人读的写法：1.2k、3.4M（.0 不写）。 */
export function short(value) {
  if (value == null) return '—';
  const one = (number, unit) => `${number.toFixed(1).replace(/\.0$/, '')}${unit}`;
  if (value >= 1e6) return one(value / 1e6, 'M');
  if (value >= 1e3) return one(value / 1e3, 'k');
  return String(Math.round(value));
}

/** 和前一段比：↑12%、↓5%、前一段是零的不写。 */
export function delta(now, before) {
  if (now == null || before == null || before === 0) return null;
  const change = Math.round(((now - before) / before) * 100);
  return { text: `${change >= 0 ? '↑' : '↓'}${Math.abs(change)}%`, up: change >= 0 };
}

/** 一格数：名字、值、和前一段比。 */
export function tile(label, value, compare) {
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

/** 一张带列名的小表（施工 I-106：原来一行「名字 · 数 · 数」挤在一起读不清）：`columns` 是列名（第一列是名字，后面的数靠右），
 *  `rows` 是每一行的格（节点或字）。名字太长的截断，悬停看全。 */
function table(title, columns, rows) {
  const box = element('div', 'stats-list');
  if (title) box.append(element('h3', null, title));
  if (rows.length === 0) {
    box.append(element('p', 'faint', say('stats/none')));
    return box;
  }
  const grid = element('div', 'stats-table');
  grid.style.gridTemplateColumns = `minmax(0, 1fr) repeat(${columns.length - 1}, auto)`;
  columns.forEach((label, at) => grid.append(element('span', at ? 'th num' : 'th', label)));
  for (const cells of rows) {
    cells.forEach((cell, at) => {
      const shown = typeof cell === 'string' ? element('span', null, cell) : cell;
      shown.classList.add(at ? 'num' : 'name');
      if (!at && typeof cell === 'string') shown.title = cell;
      grid.append(shown);
    });
  }
  box.append(grid);
  return box;
}

/** 竖条图：`rows` 是 [标签, [每一段的值…], 悬停的字?]，段照 `classes` 上色、从下往上叠。 */
export function bars(title, rows, classes) {
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
export function legend(keys, classes) {
  const box = element('div', 'legend');
  keys.forEach((key, index) => {
    const item = element('span');
    item.append(element('i', classes[index]), document.createTextNode(say(key)));
    box.append(item);
  });
  return box;
}

/** token 那一段。 */
function tokens(token, context) {
  const rate = (value) => (value == null ? '—' : `${Math.round(value * 100)}%`);
  // 金额（施工 I-104）：照核心换算成你设的币种的；没价钱的、没换成的各写一行。
  const cost = token.cost || {};
  const costOf = (one) => (one.cost == null ? '—' : money(one.cost, cost.currency));
  const tiles = element('div', 'stat-tiles');
  tiles.append(
    tile(say('stats/tokens'), short(token.total), delta(token.total, token.prev_total)),
    tile(say('stats/cost'), money(cost.amount, cost.currency), delta(cost.amount, token.prev_cost)),
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
  // 每天的花费：有一天有金额才画。
  const spent = several && token.days.some((day) => day.cost)
    ? bars(say('stats/daily-cost'), token.days.map((day) => [day.day.slice(5), [day.cost || 0], `${day.day.slice(5)} · ${money(day.cost, cost.currency)}`]), ['p2'])
    : document.createDocumentFragment();
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
    table(
      say('stats/by-purpose'),
      [say('stats/col-purpose'), say('stats/col-requests'), say('stats/col-tokens'), say('stats/col-cost')],
      (token.purposes || []).map((one) => [purpose(one.name), short(one.requests), short(one.tokens), costOf(one)]),
    ),
    table(
      say('stats/by-venue'),
      [say('stats/col-venue'), say('stats/col-tokens'), say('stats/col-cost')],
      (token.venues || []).map((one) => [venueLink(one.name, context), short(one.tokens), costOf(one)]),
    ),
    table(
      say('stats/by-model'),
      [say('stats/col-model'), say('stats/col-requests'), say('stats/col-tokens'), say('stats/col-cost')],
      (token.models || []).map((one) => [one.name || '—', short(one.requests), short(one.tokens), costOf(one)]),
    ),
  );
  return section(say('stats/token'), tiles, unpriced(cost), days, spent, make, grid);
}

/** 金额照币种写：不到 1 的多留两位（$0.0832）；没有的写「—」，认不得的币种照「0.08 XYZ」写。 */
export function money(amount, currency) {
  if (amount == null || !currency) return '—';
  const digits = Math.abs(amount) < 1 ? 4 : 2;
  try {
    return new Intl.NumberFormat(document.documentElement.lang || undefined, {
      style: 'currency',
      currencyDisplay: 'narrowSymbol',
      currency,
      minimumFractionDigits: 2,
      maximumFractionDigits: digits,
    }).format(amount);
  } catch {
    return `${amount.toFixed(digits)} ${currency}`;
  }
}

/** 金额底下的小字：另有几次请求没记录金额，另有多少没换成你设的币种。都没有的是空的。 */
export function unpriced(cost) {
  const box = element('div', 'stats-note');
  if (cost.unpriced > 0) box.append(element('p', 'faint', say('stats/unpriced', { count: cost.unpriced.toLocaleString() })));
  const rest = (cost.unconverted || []).map((one) => money(one.amount, one.currency));
  if (rest.length > 0) box.append(element('p', 'faint', say('stats/unconverted', { amounts: rest.join(say('plugins-page/separator')), currency: cost.currency || '' })));
  return box.childElementCount > 0 ? box : document.createDocumentFragment();
}

/** 消息那一段。 */
function messages(shown, context) {
  const now = shown.now;
  const before = shown.prev;
  const rate = now.received ? `${Math.round((now.replied / now.received) * 100)}%` : '—';
  const tiles = element('div', 'stat-tiles');
  tiles.append(
    tile(say('stats/received'), short(now.received), delta(now.received, before.received)),
    tile(say('stats/replied'), short(now.replied), delta(now.replied, before.replied)),
    tile(say('stats/reply-rate'), rate, null),
    tile(say('stats/recorded'), short(now.recorded), delta(now.recorded, before.recorded)),
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
  const grid = element('div', 'stats-grid');
  grid.append(
    table(
      say('stats/busiest'),
      [say('stats/col-venue'), say('stats/col-received')],
      (shown.venues || []).map((one) => [venueLink(one.name, context), short(one.received)]),
    ),
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
