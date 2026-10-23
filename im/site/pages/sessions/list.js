// 平台会话页的列表（施工 I-34 下）：一个连接一组（平台、机器人的头像、名字和号），组里列这个连接收进来的会话（头像、群名或人名、号、
// 群聊还是私聊），从新到旧；认不出是哪个连接的放在最后一组。点一个进它的记录（#sessions/<会话编号>）。能搜名字或号。
import { element, say } from '../../shell/i18n.js';
import { face } from '../venues/face.js';
import { idWord } from '../venues/names.js';
import { throttled } from './sessions.js';

/** 在 `view` 里画列表：`request` 调宿主的方法。交回 update(connections)、dispose()。 */
export function mountList(view, { request }) {
  let disposed = false;
  let answer = null;
  let bots = new Map();
  let query = '';
  const head = element('div', 'view-head sessions-list-head');
  const search = element('input', 'input venue-search');
  search.placeholder = say('sessions/search');
  search.addEventListener('input', () => {
    query = search.value.trim().toLowerCase();
    draw();
  });
  head.append(element('h1', null, say('sessions/title')), element('span', 'grow'), search);
  const body = element('div', 'sessions-groups');
  view.replaceChildren(head, body);

  const load = async () => {
    try {
      answer = await request('sessions.list');
    } catch {
      if (!answer) body.replaceChildren(element('p', 'muted', say('sessions/unreachable')));
      return;
    }
    if (!disposed) draw();
  };
  const reload = throttled(10000, load);

  const draw = () => {
    if (!answer) return;
    const groups = (answer.connections || [])
      .map((group) => ({ ...group, sessions: group.sessions.filter(matches) }))
      .filter((group) => group.sessions.length > 0);
    if (groups.length === 0) {
      body.replaceChildren(element('p', 'muted', say(query ? 'sessions/none-found' : 'sessions/none')));
      return;
    }
    body.replaceChildren(...groups.map(drawGroup));
  };
  const matches = (one) =>
    !query || String(one.name || '').toLowerCase().includes(query) || String(one.id || '').includes(query);

  const drawGroup = (group) => {
    const section = element('section', 'sessions-group');
    const title = element('div', 'sessions-group-title');
    if (group.bot) {
      // 机器人的名字问到了写名字、号写在后面；没问到的只写号。
      const bot = bots.get(String(group.bot));
      const id = `${idWord('private', group.platform)} ${group.bot}`;
      title.append(face('private', group.bot, bot?.name || String(group.bot)));
      title.append(element('strong', null, bot?.name || id));
      if (bot?.name) title.append(element('span', 'muted', id));
    } else {
      title.append(element('strong', null, say('sessions/unknown-connection')));
    }
    const list = element('div', 'sessions-list');
    for (const one of group.sessions) list.append(drawRow(one, group.platform));
    section.append(title, list);
    return section;
  };
  const drawRow = (one, platform) => {
    const row = element('button', 'sessions-row');
    const type = one.kind === 'group' ? 'group' : 'private';
    const name = one.name || (one.id ? String(one.id) : one.venue);
    const text = element('span', 'grow');
    text.append(element('span', 'name', name));
    const id = one.id ? `${idWord(type, platform)} ${one.id}` : one.venue;
    text.append(element('span', 'id', `${say(`sessions/kind-${type}`)} · ${id}`));
    row.append(face(type, one.id, name), text);
    row.addEventListener('click', () => {
      location.hash = `sessions/${encodeURIComponent(one.session)}`;
    });
    return row;
  };

  reload.call();
  return {
    update(connections) {
      bots = new Map((connections || []).map((one) => [String(one.bot), one]));
      draw();
      reload.call();
    },
    dispose() {
      disposed = true;
      reload.stop();
    },
  };
}
