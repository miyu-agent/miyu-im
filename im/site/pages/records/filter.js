// 消息记录页的搜（施工 C-1 中下，设计 04 第一节第 3 条）：关键词、人（号或名字）、从哪天到哪天，写了哪一样就照哪一样搜，都空了回到
// 原来的样子。列表页跨场所搜，一个场所的页只在那里搜（records.search 带 venue）。搜到的新的在前，点一条去它所在的场所、从那一条往回看。
import { element, say } from '../../shell/i18n.js';
import { drawItem } from './items.js';
import { openMedia } from './media.js';
import { openVenue } from './shared.js';

/** 一天的毫秒数。 */
const DAY = 86400000;

/** 搜的那一行：条件变了（停手 300 毫秒后）调 `changed(conditions)`，都空着的时候是 null。交回 { element, conditions() }。 */
export function filterBar(changed) {
  const bar = element('div', 'records-filter');
  const word = field('text', 'records/filter-word', 'grow');
  const person = field('text', 'records/filter-person');
  const from = field('date', 'records/filter-from');
  const to = field('date', 'records/filter-to');
  const clear = element('button', 'btn ghost', say('records/filter-clear'));
  clear.hidden = true;
  const conditions = () => {
    const found = {};
    if (word.value.trim()) found.query = word.value.trim();
    if (person.value.trim()) found.sender = person.value.trim();
    // 日子照本地时间：从那天零点，到「到」那天的第二天零点。
    if (from.value) found.from = new Date(`${from.value}T00:00`).getTime();
    if (to.value) found.to = new Date(`${to.value}T00:00`).getTime() + DAY;
    return Object.keys(found).length > 0 ? found : null;
  };
  let timer = 0;
  const fire = () => {
    clearTimeout(timer);
    timer = setTimeout(() => {
      const now = conditions();
      clear.hidden = !now;
      changed(now);
    }, 300);
  };
  for (const one of [word, person]) one.addEventListener('input', fire);
  for (const one of [from, to]) one.addEventListener('change', fire);
  clear.addEventListener('click', () => {
    for (const one of [word, person, from, to]) one.value = '';
    fire();
  });
  bar.append(word, person, label('records/filter-from', from), label('records/filter-to', to), clear);
  return { element: bar, conditions };
}

/** 一格：`kind` 是 text、date，`hint` 是提示的字。 */
function field(kind, hint, extra) {
  const input = element('input', `input${extra ? ` ${extra}` : ''}`);
  input.type = kind;
  if (kind === 'text') input.placeholder = say(hint);
  else input.title = say(hint);
  return input;
}

/** 日子那一格前面写「从」「到」。 */
function label(key, input) {
  const box = element('label', 'records-filter-date');
  box.append(element('span', 'muted', say(key)), input);
  return box;
}

/**
 * 照 `conditions` 搜，画进 `body`：`venue` 不空的只在那里搜；`who(sender, name)` 认人，`venueName(venue)` 是场所的名字（跨场所搜的时候写在
 * 每一条上面）；`stale()` 真的是条件又变了，这一次的不画了。
 */
export async function drawFound(body, { request, conditions, venue, who, venueName, stale }) {
  let answer;
  try {
    answer = await request('records.search', venue ? { ...conditions, venue } : conditions);
  } catch {
    if (!stale()) body.replaceChildren(element('p', 'muted', say('records/unreachable')));
    return;
  }
  if (stale()) return;
  const items = answer.items || [];
  if (items.length === 0) {
    body.replaceChildren(element('p', 'muted', say('records/none-found')));
    return;
  }
  const count = element('p', 'muted records-found-count', say(items.length >= 200 ? 'records/found-most' : 'records/found', { count: items.length }));
  const rows = items.map((item) => {
    const row = element('button', 'records-found');
    if (!venue) row.append(element('div', 'records-found-venue', venueName(item.venue)));
    const open = (found, one) => openMedia(request, { venue: found.venue, msg: found.msg, id: one.id, name: one.name });
    const drawn = drawItem(item, { who, quoted: () => null, open });
    row.append(drawn);
    row.addEventListener('click', () => openVenue(item.venue, item.id));
    return row;
  });
  body.replaceChildren(count, ...rows);
}
