// 数据统计（施工 I-14 起是一页，I-85 下并进首页，I-100 又拆回侧栏：项目主人 11-02 定首页只留今天的几个数，点进来看明细；设计 01
// 第八节第 4 条：看统计主要是看 token，放最上面）：今天、7 天、30 天，从上往下：token、消息、运行。数照宿主的 stats（核心的用量账本、
// 宿主自己记的、进程的），每一样和前一段同样长的比。金额照核心换算成你设的币种的写，没价钱的请求写「另有几次请求没记录金额」（施工
// I-104，项目主人 11-02 改了原来的「算不准不显示」）。
//
// - 连接不止一个时标题旁一个下拉（施工 I-106，项目主人 11-02：按连接切换整页，取代 I-100 的「按连接」那一段），同消息记录：有
//   「全部」，跟着整个页面选中的连接（shell/connection.js）。「全部」时场所前面画它的平台标志。
// - 场所照首页给的名字写（cards.get 的群、好友），没有的写编号；点了去会话页打开那个场所的浮窗（#venues/<种类>/<号>，带上它的连接）。
// - 图都是条：每天的 token 三样叠着画，构成画成一条，按小时 24 根。列表是带列名的小表（施工 I-106）。
// - 施工 I-272（照网页包的用量页，主人 11-22）：时段多「全部」「自定义」（两个日子，含头含尾），默认 30 天；头上写几点更新的、一个刷新；
//   token 照网页包分三样（缓存命中、没命中的输入、输出：写缓存算进没命中的输入），颜色也照它（命中浅、没命中深）；几格数的名字、先后、
//   底下一行小字照它；按用途、按模型多一列命中，模型名不带供应商、供应商灰字接在后面。
import { element, icon, say } from '../../shell/i18n.js';
import { dateField, ymd } from '../../shell/date-pick.js';
import { sect } from '../../shell/sect.js';
import { trend } from './trend.js';
import { peopleRank, venueRank } from './rankings.js';
import { connectionPick, platformDot } from '../../shell/connection-pick.js';
import { chooseConnection, chosenOf, onConnection } from '../../shell/connection.js';
import { namesOf } from '../home/activity.js';
import { venueParts } from '../records/shared.js';

const PERIODS = ['today', '7d', '30d', 'all', 'custom'];
const REMEMBER = 'miyu-im-stats-period';
const RANGE = 'miyu-im-stats-range';

/** token 的三样（照网页包的用量页）：名字和颜色。核心给的四样（读缓存、没命中、写缓存、输出）照 `three` 并成这三样。 */
export const PART_KEYS = ['stats/part-hit', 'stats/part-miss', 'stats/part-output'];
export const PART_CLASSES = ['p1', 'p0', 'p2'];

/** 四样并成三样：写缓存算进没命中的输入。 */
export function three(parts) {
  const [read = 0, uncached = 0, write = 0, output = 0] = parts || [];
  return [read, uncached + write, output];
}
// 宿主推了变了，隔多久才再问（毫秒）。
const AGAIN = 30000;

/** 在 `view` 里起数据统计：`request` 调宿主的方法。交回 update(connections)、dispose()。 */
export function mountStats(view, { request }) {
  let disposed = false;
  let period = remembered();
  let range = rememberedRange();
  let connections = [];
  let names = new Map();
  let shown = null;
  let asked = 0;
  let loadedAt = 0;
  // 上一次问的连接（「全部」是空的）。
  let scope = null;
  // 走势切到哪一样、两张排行搜了什么怎么排（施工 I-169）：重画、换时段时照旧。
  const kept = { trend: {}, people: {}, venues: {} };
  const seg = element('div', 'seg');
  const pickSlot = element('span', 'pf-slot');
  const head = element('div', 'view-head stats-head');
  const updated = element('span', 'faint stats-updated');
  const refresh = element('button', 'icon-button');
  refresh.title = say('stats/refresh');
  refresh.append(icon('i-refresh'));
  refresh.addEventListener('click', () => load());
  head.append(element('h1', null, say('nav/stats')), pickSlot, element('span', 'grow'), updated, refresh, seg);
  // 自定义的两个日子：选了「自定义」才出来。
  const from = dateField({ before: true });
  const until = dateField({ before: true });
  from.value = range.from;
  until.value = range.until;
  const custom = element('div', 'stats-range');
  custom.append(from, element('span', 'faint', say('stats/range-to')), until);
  const pickedRange = () => {
    range = { from: from.value, until: until.value };
    if (range.from > range.until) {
      range = { from: range.until, until: range.from };
      from.value = range.from;
      until.value = range.until;
    }
    try {
      localStorage.setItem(RANGE, JSON.stringify(range));
    } catch {
      // 记不住就算了。
    }
    load();
  };
  from.addEventListener('change', pickedRange);
  until.addEventListener('change', pickedRange);
  const body = element('div', 'stats-body');
  view.replaceChildren(head, custom, body);
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
    custom.hidden = period !== 'custom';
  };
  const draw = () => {
    const picked = chosenOf(connections);
    pickSlot.replaceChildren();
    if (connections.length > 1) pickSlot.append(connectionPick(connections, picked, { every: true, onPick: chooseConnection }));
    if (!shown) return;
    const context = { names, connections, tagged: !picked && connections.length > 1 };
    body.replaceChildren(
      tokens(shown.token),
      messages(shown.messages),
      trend(shown, kept.trend),
      peopleRank(shown.people || [], kept.people),
      venueRank(shown, context, kept.venues),
      running(shown.running),
    );
  };
  const load = async () => {
    const mine = ++asked;
    const picked = chosenOf(connections);
    scope = picked?.id || '';
    if (!shown) body.replaceChildren(element('p', 'faint', say('stats/loading')));
    let answer;
    try {
      const asked = period === 'custom' ? { period, from: range.from, until: range.until } : { period };
      answer = await request('stats', picked ? { ...asked, connection: picked.id } : asked);
    } catch {
      if (!disposed && mine === asked && !shown) body.replaceChildren(element('p', 'faint', say('stats/failed')));
      return;
    }
    // 等的时候又换了时段、连接的：这一份不要了。
    if (disposed || mine !== asked) return;
    shown = answer;
    loadedAt = Date.now();
    const at = new Date(loadedAt);
    updated.textContent = say('stats/updated', { time: `${String(at.getHours()).padStart(2, '0')}:${String(at.getMinutes()).padStart(2, '0')}` });
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

/** 记着的那一段；没记过、记不住的是 30 天（同网页包）。 */
function remembered() {
  try {
    const saved = localStorage.getItem(REMEMBER);
    if (PERIODS.includes(saved)) return saved;
  } catch {
    // 记不住就算了。
  }
  return '30d';
}

/** 记着的自定义的两个日子；没记过的是最近 7 天。 */
function rememberedRange() {
  try {
    const saved = JSON.parse(localStorage.getItem(RANGE) || 'null');
    if (/^\d{4}-\d{2}-\d{2}$/.test(saved?.from) && /^\d{4}-\d{2}-\d{2}$/.test(saved?.until)) return saved;
  } catch {
    // 记不住、读不懂就算了。
  }
  const now = new Date();
  const week = new Date(now.getFullYear(), now.getMonth(), now.getDate() - 6);
  return { from: ymd(week), until: ymd(now) };
}

/** 一个场所（`qq:group:777`）：名字和去浮窗的链接（带上它的连接：选中的那一个，「全部」时照平台、号找）；`tagged` 的前面画平台
 *  标志。 */
export function venueLink(id, { names, connections, tagged }) {
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

/** 一格数：名字、值、和前一段比，`sub` 是底下一行小字（施工 I-272，照网页包）。 */
export function tile(label, value, compare, sub) {
  const box = element('div', 'stat-tile');
  box.append(element('span', 'faint', label), element('b', null, value));
  const foot = element('small', 'stat-sub');
  if (sub) foot.append(document.createTextNode(sub));
  if (compare) foot.append(element('span', compare.up ? 'up' : 'down', compare.text));
  if (sub || compare) box.append(foot);
  return box;
}

/** 一段：标题和几块。 */
function section(title, ...parts) {
  // 一段一张共用的卡片（shell/sect.js，设计 11 G1）。
  const card = sect(title);
  card.body.append(...parts);
  card.box.classList.add('stats-section');
  return card.box;
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
  if (title) box.append(element('h3', null, title));
  const max = Math.max(1, ...rows.map(([, values]) => values.reduce((sum, one) => sum + one, 0)));
  // 纵轴的刻度（施工 I-169：原来看不出一根条是多少）：最高那条的数写在左上，一条淡线；悬停每根条照旧写数。
  const scale = element('div', 'bars-scale');
  scale.append(element('small', null, short(max)));
  box.append(scale);
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

/** token 那一段：几个数、构成、按用途、按模型（每天的并进走势 trend.js，按场所并进「按会话」rankings.js，施工 I-169）。 */
function tokens(token) {
  const rate = (value) => (value == null ? '—' : `${Math.round(value * 100)}%`);
  // 金额（施工 I-104）：照核心换算成你设的币种的，没换成的接在后面（施工 I-181）；没价钱的写一行。
  const cost = token.cost || {};
  const costOf = (one) => (one.cost == null ? '—' : money(one.cost, cost.currency));
  const [hit, miss, output] = three(token.parts);
  const tiles = element('div', 'stat-tiles');
  tiles.append(
    tile(say('stats/cost'), spent(cost), delta(cost.amount, token.prev_cost)),
    tile(say('stats/requests'), say('stats/requests-count', { n: short(token.requests) }), delta(token.requests, token.prev_requests),
      say('stats/venues-count', { n: (token.venues || []).length })),
    tile(say('stats/tokens'), short(token.total), delta(token.total, token.prev_total),
      say('stats/tokens-split', { input: short(hit + miss), output: short(output) })),
    tile(say('stats/hit-rate'), rate(token.hit_rate), null, say('stats/hits-count', { n: short(hit) })),
    tile(say('stats/per-reply'), short(token.per_reply), null),
  );
  // 一个 token 都没有的不画构成、按小时（空条像坏了）。
  const total = hit + miss + output;
  const make = total > 0 ? element('div', 'stats-chart') : document.createDocumentFragment();
  const strip = element('div', 'strip');
  [hit, miss, output].forEach((value, index) => {
    const piece = element('i', PART_CLASSES[index]);
    piece.style.width = `${(value / total) * 100}%`;
    piece.title = `${say(PART_KEYS[index])} ${short(value)}`;
    strip.append(piece);
  });
  if (total > 0) make.append(element('h3', null, say('stats/parts')), strip, legend(PART_KEYS, PART_CLASSES));
  // 会话的回合不带用途（核心的账本里是空的），是回复。
  const purpose = (name) => {
    const key = `stats/purpose-${name || 'turn'}`;
    const said = say(key);
    return said === key ? name : said;
  };
  const columns = (first) => [first, say('stats/col-requests'), say('stats/col-tokens'), say('stats/col-hit'), say('stats/col-cost')];
  const grid = element('div', 'stats-grid');
  grid.append(
    table(
      say('stats/by-model'),
      columns(say('stats/col-model')),
      (token.models || []).map((one) => [modelName(one.name), short(one.requests), short(one.tokens), rate(one.hit_rate), costOf(one)]),
    ),
    table(
      say('stats/by-purpose'),
      columns(say('stats/col-purpose')),
      (token.purposes || []).map((one) => [purpose(one.name), short(one.requests), short(one.tokens), rate(one.hit_rate), costOf(one)]),
    ),
  );
  return section(say('stats/token'), tiles, unpriced(cost), make, grid);
}

/** 模型那一格（照网页包）：`供应商/模型` 写成模型名，供应商灰字接在后面；悬停看全。 */
function modelName(name) {
  if (!name) return '—';
  const at = name.indexOf('/');
  const cell = element('span', 'stats-model');
  cell.title = name;
  if (at <= 0) {
    cell.textContent = name;
    return cell;
  }
  cell.append(document.createTextNode(name.slice(at + 1)), element('em', null, name.slice(0, at)));
  return cell;
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

/** 花费那一格（施工 I-181，项目主人 11-09：没换成你设的币种的不另写一行，用加号接在后面）：换成了的那一笔，再照各自的币种接上没换成的，
 *  例如「$7.32 + ¥0.42」。全没换成的不写「$0.00 +」；什么都没有的写「—」。 */
export function spent(cost) {
  const rest = (cost?.unconverted || []).map((one) => money(one.amount, one.currency));
  const converted = cost?.amount != null && cost.currency && !(cost.amount === 0 && rest.length > 0);
  const parts = converted ? [money(cost.amount, cost.currency), ...rest] : rest;
  return parts.length > 0 ? parts.join(' + ') : '—';
}

/** 金额底下的小字：另有几次请求没记录金额。没有的是空的。 */
export function unpriced(cost) {
  const box = element('div', 'stats-note');
  if (cost.unpriced > 0) box.append(element('p', 'faint', say('stats/unpriced', { count: cost.unpriced.toLocaleString() })));
  return box.childElementCount > 0 ? box : document.createDocumentFragment();
}

/** 消息那一段：几个数、按小时收到的（每天的并进走势，最热闹的并进「按会话」，施工 I-169）。 */
function messages(shown) {
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
  const hours = (shown.hours || []).some((count) => count > 0)
    ? bars(say('stats/hours'), shown.hours.map((count, hour) => [String(hour), [count], `${hour}:00 · ${say('stats/received')} ${count}`]), ['p1'])
    : document.createDocumentFragment();
  return section(say('stats/messages'), tiles, hours);
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
