// 消息记录页里一个场所的记录（施工 C-1 中下，设计 04 第一节第 3 条）：读 im 自己的库（records.page），从旧到新画人说的、她说的、群里的
// 事件（items.js），人说的底下一行回没回、为什么。往上「更早的」接着读前一页；宿主推了变了，重读最新一页、换掉里面的几条。
// 从搜到的一条点进来的（`at` 是它的行号）从那一条往回看、把它标出来，这时候不跟着新消息走，底下「回到最新」。
//
// 上面一行在这个场所里搜（filter.js）。右上角两样删：「删除记录」开删的浮窗（purge.js，这个场所，能选时间段和人）；「删除 AI 会话」删的
// 是她在这里的上下文（桥的场所会话，施工 I-34），和记录无关，点两次才删，桥认不出这个场所的会话时不出现。
import { element, icon, say } from '../../shell/i18n.js';
import { toast } from '../../shell/dialog.js';
import { face } from '../venues/face.js';
import { drawFound, filterBar } from './filter.js';
import { drawItem } from './items.js';
import { openMedia } from './media.js';
import { openPurge } from './purge.js';
import { botNames, namer, openVenue, throttled, venueParts } from './shared.js';
import { detailOf, titleOf } from './venues.js';
import { goTo } from '../../shell/connection.js';

/** 在 `view` 里画场所 `venue` 的记录，`at` 是先看哪一条（行号，空的看最新的）。交回 update(connections)、dispose()。 */
export function mountVenue(view, { request, venue, at }) {
  let disposed = false;
  // 读到的几项：键 → 项；消息另照平台编号找（引用）。
  const items = new Map();
  const byMsg = new Map();
  const people = new Map();
  let bots = new Map();
  // 前一页从哪里读；没有更早的是空的。
  let before = null;
  let loadedOnce = false;
  let searching = null;
  let asked = 0;
  const row = { venue, ...venueParts(venue), name: null };

  const head = element('div', 'view-head records-head');
  const back = element('button', 'icon-button');
  back.append(icon('i-left'));
  back.addEventListener('click', () => {
    goTo('records');
  });
  const title = element('span', 'grow records-venue-title');
  const purge = element('button', 'btn ghost danger', say('records/purge'));
  purge.addEventListener('click', () => {
    openPurge({ request, venues: [{ venue, title: titleOf(row), detail: detailOf(row) }], chosen: [venue], done: reset });
  });
  const forget = element('button', 'btn ghost danger', say('records/forget'));
  forget.title = say('records/forget-hint');
  forget.hidden = true;
  const actions = element('span', 'records-actions');
  actions.append(purge, forget);
  head.append(back, title, actions);
  const filter = filterBar((conditions) => {
    searching = conditions;
    draw();
  });
  const older = element('button', 'btn ghost records-older', say('records/older'));
  older.hidden = true;
  older.addEventListener('click', () => page(before));
  const list = element('div', 'records-timeline');
  const latest = element('button', 'btn ghost records-older', say('records/latest'));
  latest.hidden = !at;
  latest.addEventListener('click', () => openVenue(venue));
  view.replaceChildren(head, filter.element, older, list, latest);

  const drawTitle = () => {
    const name = titleOf(row);
    const text = element('span', 'records-venue-name');
    text.append(element('strong', null, name), element('span', 'muted', detailOf(row)));
    title.replaceChildren(face(row.type, row.id, name), text);
  };
  drawTitle();

  // 桥认得的名字、她在这里的会话（删上下文用）。
  let session = null;
  const known = async () => {
    let listed;
    try {
      listed = await request('sessions.list');
    } catch {
      return;
    }
    if (disposed) return;
    for (const group of listed.connections || []) {
      const found = (group.sessions || []).find((one) => one.venue === venue);
      if (found) {
        session = found.session;
        if (found.name) row.name = found.name;
      }
    }
    forget.hidden = !session;
    drawTitle();
  };
  let armed = false;
  forget.addEventListener('click', async () => {
    if (!armed) {
      armed = true;
      forget.textContent = say('records/delete-again');
      return;
    }
    forget.disabled = true;
    try {
      await request('sessions.delete', { session });
    } catch {
      forget.disabled = false;
      toast(say('records/delete-failed'));
      return;
    }
    toast(say('records/forgotten'));
    forget.hidden = true;
  });

  /** 读一页：`from` 是空的读最新一页（换掉里面的几条），不空的读它前面那一页。 */
  const page = async (from) => {
    let answer;
    try {
      answer = await request('records.page', from ? { venue, before: from } : { venue });
    } catch {
      if (!loadedOnce) list.replaceChildren(element('p', 'muted', say('records/unreachable')));
      return;
    }
    if (disposed) return;
    for (const item of answer.items || []) keep(item);
    if (from || !loadedOnce) {
      before = answer.more ? answer.before : null;
      older.hidden = !before;
    }
    const scroller = view.parentElement;
    const atBottom = scroller ? scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight < 40 : true;
    const height = scroller?.scrollHeight || 0;
    if (!searching) draw();
    if (!loadedOnce && at) list.querySelector('.records-msg.marked')?.scrollIntoView({ block: 'center' });
    else if (!loadedOnce || (!from && atBottom)) scroller?.scrollTo({ top: scroller.scrollHeight });
    else if (from && scroller) scroller.scrollTop += scroller.scrollHeight - height;
    loadedOnce = true;
  };
  const keep = (item) => {
    const key = item.type === 'event' ? `e${item.at}:${item.kind}:${item.target}:${item.actor}` : `m${item.id}`;
    items.set(key, item);
    if (item.type === 'event') return;
    byMsg.set(item.msg, item);
    if (item.name && !item.mine) people.set(item.sender, item.name);
    if (!item.mine && row.type === 'private' && item.name && !row.name) {
      row.name = item.name;
      drawTitle();
    }
  };
  const refresh = throttled(1500, () => page(null));
  /** 删过记录：清掉重读。 */
  const reset = () => {
    items.clear();
    byMsg.clear();
    before = null;
    loadedOnce = false;
    page(null);
  };

  const draw = () => {
    if (searching) {
      const ticket = ++asked;
      const stale = () => disposed || ticket !== asked;
      older.hidden = true;
      latest.hidden = true;
      drawFound(list, { request, conditions: searching, venue, who: namer(bots, people), venueName: () => '', stale });
      return;
    }
    asked += 1;
    older.hidden = !before;
    latest.hidden = !at;
    const sorted = [...items.values()].sort((a, b) => a.at - b.at || (a.id ?? Infinity) - (b.id ?? Infinity));
    const context = {
      who: namer(bots, people),
      quoted: (msg) => byMsg.get(msg) || null,
      open: (item, one) => openMedia(request, { venue, msg: item.msg, id: one.id, name: one.name }),
    };
    const shown = sorted.map((item) => {
      const drawn = drawItem(item, context);
      if (drawn && at && item.id === at) drawn.classList.add('marked');
      return drawn;
    }).filter(Boolean);
    list.replaceChildren(...(shown.length > 0 ? shown : [element('p', 'muted', say('records/empty'))]));
  };

  known();
  page(at ? at + 1 : null);
  return {
    update(connections) {
      bots = botNames(connections);
      if (!loadedOnce) return;
      if (at) {
        if (!searching) draw();
      } else refresh.call();
    },
    dispose() {
      disposed = true;
      refresh.stop();
    },
  };
}
