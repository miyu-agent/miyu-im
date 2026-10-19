// 会话页（施工 I-7 上，设计 01 第八节第 3 条）：卡片墙。头上筛「全部」「群聊」「私聊」（带张数）、搜名字或号码、「新建」；墙上分三段：
// 名单以外（「陌生群」「陌生私聊」）、群聊名单、私聊白名单。一张卡一个会话：头像和平台角标、名字、号（连着不止一个机器人时写是哪个）、
// 是否回复的开关、几个小标签；点卡打开它的浮窗。
//
// - 卡片照宿主的 cards.get 画；宿主推 cards（卡片、白名单变了）再问，开着的浮窗跟着换成新的那一张。机器人连上、断开了也再问一遍
//   （AI 在的群、好友跟着机器人）。
// - 切平台的下拉（设计：连着不止一个平台时才有）这一步不画：一份 im 只跑一个驱动，页面服务只有这一个平台。
import { element, icon, say } from '../../shell/i18n.js';
import { chips } from './chips.js';
import { openCreate } from './create.js';
import { face } from './face.js';
import { entries } from './model.js';
import { idLine, titleOf } from './names.js';
import { openVenue, toggle } from './venue.js';

const FILTERS = ['all', 'group', 'private'];

/** 墙上的三段：标题、哪几种卡。 */
const SECTIONS = [
  { label: 'venues/outside', kinds: ['other_groups', 'other_privates'] },
  { label: 'venues/groups', kinds: ['group'] },
  { label: 'venues/privates', kinds: ['private'] },
];

/** 在 `view` 里起会话页：`request` 调宿主的方法。交回 update(connections)、event(name)、dispose()。 */
export function mountVenues(view, { request }) {
  let answer = null;
  let all = [];
  let filter = 'all';
  let query = '';
  let open = null;
  let platform = '';
  let bots = new Map();
  // 连着的机器人（号，逗号隔开）：还没收到过连接的是空的。
  let seen = null;

  const head = element('div', 'view-head');
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
    if (!answer) return;
    const listed = new Set(all.filter((one) => one.id).map((one) => `${one.type}:${one.id}`));
    openCreate({ request, platform, answer, listed }, async (type, id) => {
      await load();
      const added = all.find((one) => one.type === type && one.id === id);
      if (added) show(added);
    });
  });
  head.append(element('h1', null, say('nav/venues')), seg, element('span', 'grow'), search, create);
  const wall = element('div', 'wall');
  view.replaceChildren(head, wall);

  const load = async () => {
    try {
      const [cards, status] = await Promise.all([request('cards.get'), platform ? null : request('status')]);
      answer = cards;
      if (status) platform = status.platform || '';
    } catch {
      // 读不到的：还没读到过的写一行，读到过的照旧画。
      if (!answer) wall.replaceChildren(element('p', 'faint wall-none', say('venues/failed')));
      return;
    }
    all = entries(answer);
    drawSeg();
    drawWall();
    if (open) {
      const now = all.find((one) => one.kind === open.kind && one.id === open.id);
      if (now) open.dialog.update(now);
      else open.dialog.close();
    }
  };

  const drawSeg = () => {
    seg.replaceChildren();
    for (const id of FILTERS) {
      const count = all.filter((one) => id === 'all' || one.type === id).length;
      const button = element('button', id === filter ? 'on' : null, say('venues/filter', { name: say(`venues/filter-${id}`), count }));
      button.addEventListener('click', () => {
        filter = id;
        drawSeg();
        drawWall();
      });
      seg.append(button);
    }
  };

  const drawWall = () => {
    const personas = new Map((answer?.personas || []).map((one) => [one.persona, one.name || one.persona]));
    const context = { platform, bots };
    const shown = all.filter((one) => filter === 'all' || one.type === filter).filter((one) => matches(one, context, query));
    const parts = [];
    for (const section of SECTIONS) {
      const cards = shown.filter((one) => section.kinds.includes(one.kind));
      if (cards.length === 0) continue;
      parts.push(element('div', 'wall-label', say(section.label)));
      for (const one of cards) parts.push(card(one, context, personas));
    }
    if (answer && parts.length === 0) parts.push(element('p', 'faint wall-none', say('venues/none')));
    wall.replaceChildren(...parts);
  };

  const card = (one, context, personas) => {
    const shown = element('div', one.on === false ? 'vcard off' : 'vcard');
    const top = element('div', 'top');
    const named = element('div', 'grow');
    named.append(element('div', 'name', titleOf(one)), element('div', 'id', idLine(one, context)));
    top.append(face(one, platform), named);
    if (one.on !== null) top.append(toggle(one, request));
    const tags = element('div', 'chips');
    for (const chip of chips(one, personas)) tags.append(element('span', chip.accent ? 'chip accent' : 'chip', chip.text));
    shown.append(top, tags);
    shown.addEventListener('click', () => show(one));
    return shown;
  };

  const show = (one) => {
    const opened = { kind: one.kind, id: one.id, dialog: null };
    open = opened;
    // 关了的不再跟着换。
    opened.dialog = openVenue(one, { request, platform, bots }, () => {
      if (open === opened) open = null;
    });
  };

  load();
  return {
    update(connections) {
      bots = new Map(connections.map((one) => [String(one.bot), one.name || String(one.bot)]));
      if (!platform && connections[0]) platform = connections[0].platform;
      const now = connections.filter((one) => one.connected).map((one) => one.bot).join(',');
      const again = seen !== null && now !== seen;
      seen = now;
      if (again) load();
      else drawWall();
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
