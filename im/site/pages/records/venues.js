// 消息记录页的列表（施工 C-1 中下，设计 01 第八节第 9 条）：库里有记录的场所，一个连接一组（机器人的头像、名字和号），组里从新到旧列
// 场所（头像、群名或人名、群聊还是私聊和号、最后一条什么时候、一共几条）；哪个连接收的照桥的场所会话认（sessions.list，名字也照它），
// 认不出的放在最后一组。点一个进它的记录。上面一行搜（跨场所，filter.js），右上角「删除记录」开删的浮窗（purge.js，勾几个场所）。
import { element, say } from '../../shell/i18n.js';
import { face } from '../venues/face.js';
import { idWord } from '../venues/names.js';
import { drawFound, filterBar } from './filter.js';
import { openPurge } from './purge.js';
import { botNames, namer, openVenue, throttled, venueParts, when } from './shared.js';

/** 在 `view` 里画列表：`request` 调宿主的方法。交回 update(connections)、dispose()。 */
export function mountVenues(view, { request }) {
  let disposed = false;
  let rows = null;
  let bots = new Map();
  let searching = null;
  let asked = 0;
  const head = element('div', 'view-head records-head');
  const purge = element('button', 'btn ghost danger', say('records/purge'));
  purge.addEventListener('click', () => {
    openPurge({ request, venues: (rows || []).map(choice), chosen: [], done: load });
  });
  head.append(element('h1', null, say('records/title')), element('span', 'grow'), purge);
  const filter = filterBar((conditions) => {
    searching = conditions;
    draw();
  });
  const body = element('div', 'records-groups');
  view.replaceChildren(head, filter.element, body);

  /** 库里的场所，配上桥认得的连接和名字。 */
  const load = async () => {
    let listed;
    let sessions = null;
    try {
      [listed, sessions] = await Promise.all([
        request('records.venues'),
        request('sessions.list').catch(() => null),
      ]);
    } catch {
      if (!rows) body.replaceChildren(element('p', 'muted', say('records/unreachable')));
      return;
    }
    if (disposed) return;
    const known = new Map();
    for (const group of sessions?.connections || []) {
      for (const one of group.sessions || []) known.set(one.venue, { bot: group.bot, name: one.name });
    }
    rows = (listed.venues || []).map((row) => ({ ...row, ...venueParts(row.venue), ...known.get(row.venue) }));
    if (!searching) draw();
  };
  const reload = throttled(10000, load);

  const draw = () => {
    if (searching) {
      const ticket = ++asked;
      const venueName = (venue) => titleOf(rows?.find((one) => one.venue === venue) || venueParts(venue));
      const stale = () => disposed || ticket !== asked;
      drawFound(body, { request, conditions: searching, who: namer(bots, new Map()), venueName, stale });
      return;
    }
    asked += 1;
    if (!rows) return;
    purge.disabled = rows.length === 0;
    if (rows.length === 0) {
      body.replaceChildren(element('p', 'muted', say('records/none')));
      return;
    }
    const order = [];
    const groups = new Map();
    for (const row of rows) {
      const bot = row.bot ? String(row.bot) : '';
      if (!groups.has(bot)) {
        groups.set(bot, []);
        order.push(bot);
      }
      groups.get(bot).push(row);
    }
    // 认不出连接的那一组放最后。
    order.sort((a, b) => (a === '') - (b === ''));
    body.replaceChildren(...order.map((bot) => drawGroup(bot, groups.get(bot))));
  };

  const drawGroup = (bot, list) => {
    const section = element('section', 'records-group');
    const title = element('div', 'records-group-title');
    if (bot) {
      // 机器人的名字问到了写名字、号写在后面；没问到的只写号。
      const name = bots.get(bot);
      const id = `${idWord('private', list[0].platform)} ${bot}`;
      title.append(face('private', bot, name || bot), element('strong', null, name || id));
      if (name) title.append(element('span', 'muted', id));
    } else {
      title.append(element('strong', null, say('records/unknown-connection')));
    }
    const grid = element('div', 'records-list');
    for (const row of list) grid.append(drawRow(row));
    section.append(title, grid);
    return section;
  };
  const drawRow = (row) => {
    const button = element('button', 'records-row');
    const text = element('span', 'grow');
    text.append(element('span', 'name', titleOf(row)), element('span', 'id', detailOf(row)));
    const side = element('span', 'records-row-side');
    side.append(element('span', null, when(row.last)), element('span', null, say('records/count', { count: row.count })));
    button.append(face(row.type, row.id, titleOf(row)), text, side);
    button.addEventListener('click', () => openVenue(row.venue));
    return button;
  };

  reload.call();
  return {
    update(connections) {
      bots = botNames(connections);
      if (!searching) draw();
      reload.call();
    },
    dispose() {
      disposed = true;
      reload.stop();
    },
  };
}

/** 场所的名字：问到了群名、人名写名字，没问到的写号。 */
export function titleOf(row) {
  return row.name || row.id || row.venue;
}

/** 名字下面那一行：群聊还是私聊、号。 */
export function detailOf(row) {
  return `${say(`records/kind-${row.type}`)} · ${idWord(row.type, row.platform)} ${row.id}`;
}

/** 删的浮窗里能勾的一个场所。 */
function choice(row) {
  return { venue: row.venue, title: titleOf(row), detail: detailOf(row) };
}
