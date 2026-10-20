// 会话页（施工 I-7 上，设计 01 第八节第 3 条；I-7 上修照样板按平台分）：卡片墙。一页是一个平台的会话：连着不止一个平台时头上先切平台
// （platforms-pick.js），卡上不再画平台角标；有哪几种会话、号怎么叫由平台给（platforms.js 的 venues）。再筛「全部」「群聊」「私聊」
// （张数只数名单里的，不数「陌生群」「陌生私聊」两张兜底卡）、搜名字或号码、「新建」；墙上分三段：名单以外、群聊名单、私聊白名单。
// 一张卡一个会话：头像、名字、号（连着不止一个机器人时写是哪个）、是否回复的开关、几个小标签；点卡打开它的浮窗。
//
// - 卡片照宿主的 cards.get 画；宿主推 cards（卡片、白名单变了）再问，开着的浮窗跟着换成新的那一张。机器人连上、断开了也再问一遍
//   （AI 在的群、好友跟着机器人）。
// - 重画时没变的卡照旧用原来那一张（不重新建，头像不闪），变了的才换。
// - 宿主是一个平台的（一份 im 跑一个驱动）：卡片都是宿主那个平台的，切到别的平台（以后一个宿主带几个驱动时）只看那个平台的。
import { element, icon, say } from '../../shell/i18n.js';
import { venueKinds } from '../home/platforms.js';
import { chips } from './chips.js';
import { openCreate } from './create.js';
import { face } from './face.js';
import { entries } from './model.js';
import { idLine, titleOf } from './names.js';
import { platformPick } from './platforms-pick.js';
import { fallbacksOf } from './plugins.js';
import { openVenue, toggle } from './venue.js';

/** 墙上的三段：标题、哪几种卡、属于哪一种会话。 */
const SECTIONS = [
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
  // 宿主的平台、现在看的平台、连着的机器人（号 → 名字、平台、连没连着）、出现过的平台。
  let host = '';
  let chosen = '';
  let bots = new Map();
  let platforms = [];
  // 连着的机器人（号，逗号隔开）：还没收到过连接的是空的。
  let seen = null;
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
    if (!answer || chosen !== host) return;
    const listed = new Set(all.filter((one) => one.id).map((one) => `${one.type}:${one.id}`));
    openCreate({ request, platform: chosen, answer, listed }, async (type, id) => {
      await load();
      const added = all.find((one) => one.type === type && one.id === id);
      if (added) show(added);
    });
  });
  head.append(element('h1', null, say('nav/venues')), pickSlot, seg, element('span', 'grow'), search, create);
  const wall = element('div', 'wall');
  view.replaceChildren(head, wall);

  const load = async () => {
    try {
      const [cards, status] = await Promise.all([request('cards.get'), host ? null : request('status')]);
      answer = cards;
      if (status) host = status.platform || '';
    } catch {
      // 读不到的：还没读到过的写一行，读到过的照旧画。
      if (!answer) wall.replaceChildren(element('p', 'faint wall-none', say('venues/failed')));
      return;
    }
    chosen ||= host;
    if (!platforms.includes(host)) platforms = [host, ...platforms];
    all = entries(answer);
    drawHead();
    drawWall();
    if (wanted) {
      const [type, id] = wanted.split('/');
      wanted = '';
      const one = all.find((each) => each.type === type && each.id === id);
      if (one) show(one);
      history.replaceState(null, '', '#venues');
    }
    if (open) {
      const now = all.find((one) => one.kind === open.kind && one.id === open.id);
      if (now) open.dialog.update(now);
      else open.dialog.close();
    }
  };

  /** 这个平台上的卡（宿主以外的平台还没有卡）。 */
  const mine = () => (chosen === host ? all : []);

  const drawHead = () => {
    const items = platforms.map((id) => ({
      id,
      bots: [...bots.values()].filter((bot) => bot.platform === id && bot.connected).length,
      venues: id === host ? all.filter((one) => one.id).length : null,
    }));
    pickSlot.replaceChildren();
    if (items.length > 1) {
      pickSlot.append(
        platformPick(items, chosen, (id) => {
          chosen = id;
          drawHead();
          drawWall();
        }),
      );
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
    // 只有一种会话的平台不用筛。
    seg.hidden = kinds.length < 2;
    create.disabled = chosen !== host;
  };

  const drawWall = () => {
    const personas = new Map((answer?.personas || []).map((one) => [one.persona, one.name || one.persona]));
    const context = { platform: chosen, bots: names() };
    const kinds = venueKinds(chosen);
    const shown = mine()
      .filter((one) => kinds[one.type])
      .filter((one) => filter === 'all' || one.type === filter)
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
    top.append(face(one.type, one.id, one.name), named);
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
    const context = { request, platform: chosen, bots: names(), personas, friends, plugins, fallbacks };
    opened.dialog = openVenue(one, context, () => {
      if (open === opened) open = null;
    });
  };

  /** 这个平台连着的机器人：号 → 名字（卡上写是哪个机器人）。 */
  const names = () =>
    new Map([...bots].filter(([, bot]) => bot.platform === chosen).map(([id, bot]) => [id, bot.name]));

  load();
  return {
    update(connections) {
      bots = new Map(
        connections.map((one) => [
          String(one.bot),
          { name: one.name || String(one.bot), platform: one.platform, connected: one.connected },
        ]),
      );
      platforms = [...new Set([host, ...connections.map((one) => one.platform)].filter(Boolean))];
      const now = connections.filter((one) => one.connected).map((one) => one.bot).join(',');
      const again = seen !== null && now !== seen;
      seen = now;
      if (again) load();
      else if (answer) {
        drawHead();
        drawWall();
      }
      // 宿主推了变了（来了消息）：开着的浮窗再问今天的数和最近判断。
      open?.dialog.refresh();
    },
    event(name) {
      if (name === 'cards') load();
    },
    dispose() {
      open?.dialog.close();
    },
  };
}

/** `one` 合不合搜的字 `query`（名字、号、号那一行）。 */
function matches(one, context, query) {
  if (!query) return true;
  return `${titleOf(one)} ${one.id || ''} ${idLine(one, context)}`.toLowerCase().includes(query);
}
