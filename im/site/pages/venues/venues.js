// 会话页（施工 I-7 上，设计 01 第八节第 3 条；I-85 中照设计 07 第 3.2 节按连接分）：卡片墙。一页是一个连接的会话：连接不止一个时
// 头上一个下拉切连接（shell/connection-pick.js，和首页、消息记录共用选中的那一个），没有「全部」：选的是「全部」时先请人选一个；
// 卡片照选中的连接的平台问（cards.get、cards.* 都带 platform），卡上不再画平台角标；有哪几种会话、号怎么叫由平台给（platforms.js 的
// venues）。再筛「全部」「群聊」「私聊」
// （张数只数名单里的，不数「陌生群」「陌生私聊」两张兜底卡）、搜名字或号码、「新建」；墙上分四段：默认（「会话默认值」一张，2026-10-20
// 从设置页挪过来，只在「全部」里）、名单以外、群聊名单、私聊白名单。
// 一张卡一个会话：头像、名字、号（连着不止一个机器人时写是哪个）、是否回复的开关、几个小标签；点卡打开它的浮窗。
//
// - 卡片照宿主的 cards.get 画；宿主推 cards（卡片、白名单变了）再问，开着的浮窗跟着换成新的那一张。机器人连上、断开了也再问一遍
//   （AI 在的群、好友跟着机器人）。
// - 重画时没变的卡照旧用原来那一张（不重新建，头像不闪），变了的才换。
import { element, icon, say } from '../../shell/i18n.js';
import { venueKinds } from '../home/platforms.js';
import { chips } from './chips.js';
import { openCreate } from './create.js';
import { faceOf } from './face.js';
import { entries } from './model.js';
import { idLine, titleOf } from './names.js';
import { connectionLabel, connectionPick, platformDot } from '../../shell/connection-pick.js';
import { chooseConnection, chosenOf, goTo, onConnection } from '../../shell/connection.js';
import { fallbacksOf } from './plugins.js';
import { openVenue, toggle } from './venue.js';

/** 墙上的三段：标题、哪几种卡、属于哪一种会话。 */
const SECTIONS = [
  { label: 'venues/defaults-section', kinds: ['defaults'] },
  { label: 'venues/outside', kinds: ['other_groups', 'other_privates'] },
  { label: 'venues/groups', kinds: ['group'], type: 'group' },
  { label: 'venues/privates', kinds: ['private'], type: 'private' },
];

/** 在 `view` 里起会话页：`request` 调宿主的方法。交回 update(connections)、event(name)、dispose()。 */
export function mountVenues(view, { request, path }) {
  let answer = null;
  let all = [];
  let filter = 'all';
  let query = '';
  let open = null;
  // 从别的页点过来的（#venues/group/777，施工 I-14 统计页）：读到卡以后打开它一次。
  let wanted = path || '';
  // 外壳问到的连接、选中的那一个（「全部」是空的）、它的平台（还没选的是空的）、问卡片时问的那个平台。
  let connections = [];
  let picked = null;
  let chosen = '';
  let asked = '';
  // 连着的机器人（号，逗号隔开）：还没收到过连接的是空的。
  let seen = null;
  // 卡片相关的方法、能贴的表情都带上选中的平台（施工 I-85 中、补）：浮窗、新建里也照它。
  const scoped = (method, params) =>
    request(method, method.startsWith('cards.') || method === 'reactions' ? { ...params, platform: chosen } : params);
  // 墙上画过的卡：编号 → { sig, card }。
  const drawn = new Map();

  const head = element('div', 'view-head');
  const pickSlot = element('span', 'pf-slot');
  const seg = element('div', 'seg');
  const search = element('input', 'input venue-search');
  search.placeholder = say('venues/search');
  search.addEventListener('input', () => {
    query = search.value.trim().toLowerCase();
    drawWall();
  });
  const create = element('button', 'btn primary');
  create.append(icon('i-plus'), document.createTextNode(say('venues/new')));
  create.addEventListener('click', () => {
    if (!answer || !chosen) return;
    const listed = new Set(all.filter((one) => one.id).map((one) => `${one.type}:${one.id}`));
    openCreate({ request: scoped, platform: chosen, answer, listed }, async (type, id) => {
      await load();
      const added = all.find((one) => one.type === type && one.id === id);
      if (added) show(added);
    });
  });
  head.append(element('h1', null, say('nav/venues')), pickSlot, seg, element('span', 'grow'), search, create);
  const wall = element('div', 'wall');
  view.replaceChildren(head, wall);

  const load = async () => {
    if (!chosen) {
      // 还没选连接（连接不止一个、选的是「全部」）：请人先选一个。
      answer = null;
      all = [];
      drawHead();
      wall.replaceChildren(choosePrompt());
      return;
    }
    const platform = chosen;
    asked = platform;
    let cards;
    try {
      cards = await request('cards.get', { platform });
    } catch {
      // 读不到的：还没读到过的写一行，读到过的照旧画。
      if (!answer) wall.replaceChildren(element('p', 'faint wall-none', say('venues/failed')));
      return;
    }
    // 等的时候又换了连接：这一份不要了。
    if (platform !== chosen) return;
    answer = cards;
    all = entries(answer);
    drawHead();
    drawWall();
    if (wanted) {
      const [type, id] = wanted.split('/');
      wanted = '';
      const one = all.find((each) => each.type === type && each.id === id);
      if (one) show(one);
      goTo('venues', { replace: true });
    }
    if (open) {
      const now = all.find((one) => one.kind === open.kind && one.id === open.id);
      if (now) open.dialog.update(now);
      else open.dialog.close();
    }
  };

  /** 这个连接的卡。 */
  const mine = () => all;

  /** 「全部」时请人先选一个连接（施工 I-85 中）：一句话，下面每个连接一个按钮。 */
  const choosePrompt = () => {
    const box = element('div', 'wall-none pick-prompt');
    box.append(element('p', 'faint', say('venues/pick-connection')));
    const row = element('div', 'pick-row');
    for (const one of connections) {
      const button = element('button', 'btn');
      button.append(platformDot(one.platform), document.createTextNode(connectionLabel(one)));
      button.addEventListener('click', () => chooseConnection(one.id));
      row.append(button);
    }
    box.append(row);
    return box;
  };

  const drawHead = () => {
    pickSlot.replaceChildren();
    if (connections.length > 1) {
      pickSlot.append(connectionPick(connections, picked, { every: false, onPick: chooseConnection }));
    }
    const kinds = Object.keys(venueKinds(chosen));
    if (filter !== 'all' && !kinds.includes(filter)) filter = 'all';
    seg.replaceChildren();
    for (const id of ['all', ...kinds]) {
      const count = mine().filter((one) => one.id && (id === 'all' || one.type === id)).length;
      const name = say(`venues/filter-${id}`);
      const button = element('button', id === filter ? 'on' : null, say('venues/filter', { name, count }));
      button.addEventListener('click', () => {
        filter = id;
        drawHead();
        drawWall();
      });
      seg.append(button);
    }
    // 只有一种会话的平台不用筛；还没选连接的筛、搜、新建都不画。
    seg.hidden = kinds.length < 2 || !chosen;
    search.hidden = !chosen;
    create.hidden = !chosen;
  };

  const drawWall = () => {
    const personas = new Map((answer?.personas || []).map((one) => [one.persona, one.name || one.persona]));
    const context = { platform: chosen, bots: names() };
    const kinds = venueKinds(chosen);
    const shown = mine()
      .filter((one) => one.kind === 'defaults' || kinds[one.type])
      .filter((one) => filter === 'all' || (one.type === filter && one.kind !== 'defaults'))
      .filter((one) => matches(one, context, query));
    const parts = [];
    const kept = new Set();
    for (const section of SECTIONS) {
      if (section.type && !kinds[section.type]) continue;
      const cards = shown.filter((one) => section.kinds.includes(one.kind));
      if (cards.length === 0) continue;
      parts.push(element('div', 'wall-label', say(section.label)));
      for (const one of cards) {
        const key = `${one.kind}:${one.id}`;
        const labels = chips(one, personas);
        const sig = JSON.stringify([one, titleOf(one), idLine(one, context), labels]);
        let made = drawn.get(key);
        if (!made || made.sig !== sig) {
          made = { sig, card: card(one, context, labels) };
          drawn.set(key, made);
        }
        kept.add(key);
        parts.push(made.card);
      }
    }
    for (const key of [...drawn.keys()]) if (!kept.has(key)) drawn.delete(key);
    if (answer && parts.length === 0) parts.push(element('p', 'faint wall-none', say('venues/none')));
    wall.replaceChildren(...parts);
  };

  const card = (one, context, labels) => {
    const shown = element('div', one.on === false ? 'vcard off' : 'vcard');
    const top = element('div', 'top');
    const named = element('div', 'grow');
    named.append(element('div', 'name', titleOf(one)), element('div', 'id', idLine(one, context)));
    top.append(faceOf(one), named);
    if (one.on !== null) top.append(toggle(one, request));
    const tags = element('div', 'chips');
    for (const chip of labels) tags.append(element('span', chip.accent ? 'chip accent' : 'chip', chip.text));
    shown.append(top, tags);
    shown.addEventListener('click', () => show(one));
    return shown;
  };

  const show = (one) => {
    const opened = { kind: one.kind, id: one.id, dialog: null };
    open = opened;
    // 关了的不再跟着换。
    const personas = answer?.personas || [];
    const friends = answer?.friends || [];
    // 管理插件（施工 I-7 中四）：宿主认得的插件、兜底卡上的开关。
    const plugins = answer?.plugins || [];
    const fallbacks = fallbacksOf(all);
    // 连接做得到的几样（施工 I-85 补）：卡上平台做不到的选项不列。
    const caps = picked?.caps;
    const context = { request: scoped, platform: chosen, bots: names(), personas, friends, plugins, fallbacks, caps };
    opened.dialog = openVenue(one, context, () => {
      if (open === opened) open = null;
    });
  };

  /** 这个平台连着的机器人：号 → 名字（卡上写是哪个机器人）。 */
  const names = () =>
    new Map(
      connections
        .filter((one) => one.platform === chosen)
        .map((one) => [String(one.bot), one.name || String(one.bot)]),
    );

  /** 照这一刻的连接、选中的那一个定看哪个平台；换了的关掉开着的浮窗、重问卡片。 */
  const pick = () => {
    picked = chosenOf(connections);
    const platform = picked?.platform || '';
    if (platform === chosen && (asked === platform || !platform)) return false;
    chosen = platform;
    open?.dialog.close();
    return true;
  };

  const unlisten = onConnection(() => {
    if (pick()) load();
    else drawHead();
  });
  return {
    update(now) {
      connections = now;
      const switched = pick();
      const alive = now.filter((one) => one.connected).map((one) => one.id).join(',');
      const again = seen !== null && alive !== seen;
      seen = alive;
      if (switched || again || (!answer && chosen)) load();
      else {
        drawHead();
        if (answer) drawWall();
        else if (!chosen) wall.replaceChildren(choosePrompt());
      }
      // 宿主推了变了（来了消息）：开着的浮窗再问今天的数和最近判断。
      open?.dialog.refresh();
    },
    event(name) {
      if (name === 'cards') load();
    },
    dispose() {
      unlisten();
      open?.dialog.close();
    },
  };
}

/** `one` 合不合搜的字 `query`（名字、号、号那一行）。 */
function matches(one, context, query) {
  if (!query) return true;
  return `${titleOf(one)} ${one.id || ''} ${idLine(one, context)}`.toLowerCase().includes(query);
}
