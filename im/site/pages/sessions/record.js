// 平台会话页里一个会话的记录（施工 I-34 下）：只读。画核心的视图条目（宿主的 sessions.page）：人说的（名字、时刻、字、带的东西）、
// 她的回复、几步思考和工具收起成一行、旁白；回合的收尾不画。人说的每一条底下一行小字：回没回、为什么（同会话浮窗「最近判断」的说法，
// venues/overview.js 的 reason），判官给了理由的点开看。往上「更早的」接着读前一页；宿主推了变了，重读最新一页、换掉里面的几条。
// 右上角「删除」：点一次换成「再点一次删除」，再点才删（同会话浮窗的「移出」），删了回列表。
import { element, icon, say } from '../../shell/i18n.js';
import { toast } from '../../shell/dialog.js';
import { reason } from '../venues/overview.js';
import { throttled } from './sessions.js';

/** 在 `view` 里画会话 `session` 的记录：`request` 调宿主的方法。交回 update(connections)、dispose()。 */
export function mountRecord(view, { request, session }) {
  let disposed = false;
  // 条目：编号 → 条目；先后照 order。判断、名字照编号。
  const entries = new Map();
  let order = [];
  const judged = new Map();
  const names = new Map();
  // 前一页从哪里读（`first`）；没有更早的是空的。
  let before = null;
  let loadedOnce = false;

  const head = element('div', 'view-head sessions-head');
  const back = element('button', 'icon-button');
  back.append(icon('i-left'));
  back.addEventListener('click', () => {
    location.hash = 'sessions';
  });
  const remove = element('button', 'btn ghost danger', say('sessions/delete'));
  let armed = false;
  remove.addEventListener('click', async () => {
    if (!armed) {
      armed = true;
      remove.textContent = say('sessions/delete-again');
      return;
    }
    remove.disabled = true;
    try {
      await request('sessions.delete', { session });
    } catch {
      remove.disabled = false;
      toast(say('sessions/delete-failed'));
      return;
    }
    toast(say('sessions/deleted'));
    location.hash = 'sessions';
  });
  head.append(back, element('h1', null, say('sessions/record')), element('span', 'grow'), remove);
  const older = element('button', 'btn ghost sessions-older', say('sessions/older'));
  older.hidden = true;
  older.addEventListener('click', () => page(before));
  const list = element('div', 'sessions-record');
  view.replaceChildren(head, older, list);

  /** 读一页：`from` 是空的读最新一页（换掉里面的几条、接在后面），不空的读它前面那一页（排在最前）。 */
  const page = async (from) => {
    let answer;
    try {
      answer = await request('sessions.page', from ? { session, before: from } : { session });
    } catch {
      if (!loadedOnce) list.replaceChildren(element('p', 'muted', say('sessions/unreachable')));
      return;
    }
    if (disposed) return;
    for (const [id, note] of Object.entries(answer.judged || {})) judged.set(id, note);
    for (const [id, name] of Object.entries(answer.names || {})) names.set(id, name);
    const ids = [];
    for (const entry of answer.entries || []) {
      entries.set(entry.id, entry);
      ids.push(entry.id);
    }
    const fresh = new Set(ids);
    if (from) order = [...ids, ...order.filter((id) => !fresh.has(id))];
    else order = [...order.filter((id) => !fresh.has(id)), ...ids];
    if (from || !loadedOnce) {
      before = answer.more ? answer.first : null;
      older.hidden = !before;
    }
    const atBottom = view.parentElement
      ? view.parentElement.scrollHeight - view.parentElement.scrollTop - view.parentElement.clientHeight < 40
      : true;
    draw();
    if (!loadedOnce || (!from && atBottom)) {
      view.parentElement?.scrollTo({ top: view.parentElement.scrollHeight });
    }
    loadedOnce = true;
  };
  const refresh = throttled(1500, () => page(null));

  const draw = () => {
    const shown = order.map((id) => entries.get(id)).filter((entry) => entry && !entry.hidden).map(drawEntry).filter(Boolean);
    list.replaceChildren(...(shown.length > 0 ? shown : [element('p', 'muted', say('sessions/empty'))]));
  };
  const drawEntry = (entry) => {
    switch (entry.kind) {
      case 'user':
        return drawUser(entry);
      case 'reply':
        return bubble('her', say('sessions/her'), entry.at, entry.text);
      case 'media':
        return bubble('her', say('sessions/her'), entry.at, entry.alt || say('sessions/picture'));
      case 'group':
        return element('div', 'sessions-step', (entry.summary || []).map((part) => part.text).join(''));
      case 'notice':
        return element('div', 'sessions-notice', say('sessions/notice', { what: entry.what }));
      default:
        return null;
    }
  };
  const drawUser = (entry) => {
    const who = names.get(entry.id) || entry.from?.id || say('sessions/someone');
    const parts = [entry.text || ''];
    for (const one of entry.attachments || []) parts.push(`[${one.name || one.kind}]`);
    const box = bubble('them', who, entry.at, parts.filter(Boolean).join(' '));
    const note = judged.get(entry.id);
    if (note) {
      const line = element('div', `sessions-judged ${note.outcome === 'reply' ? 'yes' : ''}`, reason(note));
      const said = note.why?.reason_text;
      if (said) {
        line.classList.add('more');
        const detail = element('div', 'sessions-judge-said', said);
        detail.hidden = true;
        line.addEventListener('click', () => {
          detail.hidden = !detail.hidden;
        });
        box.append(line, detail);
      } else box.append(line);
    }
    return box;
  };

  page(null);
  return {
    update() {
      if (loadedOnce) refresh.call();
    },
    dispose() {
      disposed = true;
      refresh.stop();
    },
  };
}

/** 一条话：`side` 是 them（人说的）、her（她说的），名字 `who`、时刻 `at`、字 `text`。 */
function bubble(side, who, at, text) {
  const box = element('div', `sessions-msg ${side}`);
  const top = element('div', 'sessions-msg-top');
  top.append(element('strong', null, who), element('span', 'muted', when(at)));
  box.append(top, element('div', 'sessions-msg-text', text));
  return box;
}

/** 时刻写成本地的月日时分。 */
function when(at) {
  const date = new Date(at);
  if (Number.isNaN(date.getTime())) return '';
  return date.toLocaleString(undefined, { month: 'numeric', day: 'numeric', hour: '2-digit', minute: '2-digit' });
}
