// 消息记录里的一项（施工 C-1 中下，设计 04 第一节第 1、3 条）：一条话一个气泡（名字、时刻、引用的那一条、字、带的东西的占位），
// 她说的靠右；撤回的划掉，底下写谁什么时候撤回的；人说的底下一行回没回、为什么（同会话浮窗「最近判断」的说法，venues/overview.js
// 的 reason），判官给了理由的点开看。群里的事件（禁言、解禁、踢人、进群、退群）是时间线上居中的一行小字，做的人照名字写；撤回已经
// 划在气泡上，不另写一行。
import { element, say } from '../../shell/i18n.js';
import { reason } from '../venues/overview.js';
import { when } from './shared.js';

/** 画 `item`（records.page、records.search 的一项）：`who(sender, name)` 认人，`quoted(msg)` 找引用的那一条（找不到的是空的）。 */
export function drawItem(item, { who, quoted }) {
  if (item.type === 'event') return drawEvent(item, who);
  const box = element('div', `records-msg ${item.mine ? 'her' : 'them'}`);
  if (item.recalled_at) box.classList.add('recalled');
  const top = element('div', 'records-msg-top');
  top.append(element('strong', null, who(item.sender, item.name)), element('span', 'muted', when(item.at)));
  box.append(top);
  if (item.reply_to) {
    const source = quoted(item.reply_to);
    const line = source
      ? say('records/quote', { who: who(source.sender, source.name), text: clip(source.text || placeholders(source)) })
      : say('records/quote-unknown');
    box.append(element('div', 'records-quote', line));
  }
  const text = element('div', 'records-msg-text', item.text || '');
  for (const one of item.media || []) {
    if (text.textContent) text.append(' ');
    text.append(element('span', 'records-media', placeholder(one)));
  }
  box.append(text);
  if (item.recalled_at) {
    const fields = { who: item.recalled_by ? who(item.recalled_by) : '', at: when(item.recalled_at) };
    box.append(element('div', 'records-recalled', say(item.recalled_by ? 'records/recalled' : 'records/recalled-plain', fields)));
  }
  if (item.judged && !item.mine) judged(box, item.judged);
  return box;
}

/** 人说的那一条底下一行回没回、为什么；判官给了理由的点开看。 */
function judged(box, note) {
  const line = element('div', `records-judged ${note.outcome === 'reply' ? 'yes' : ''}`, reason(note));
  box.append(line);
  const said = note.why?.reason_text;
  if (!said) return;
  line.classList.add('more');
  const detail = element('div', 'records-judge-said', said);
  detail.hidden = true;
  line.addEventListener('click', () => {
    detail.hidden = !detail.hidden;
  });
  box.append(detail);
}

/** 群里的一件事：居中一行小字，前面是时刻。撤回的不画（划在气泡上了）。 */
function drawEvent(item, who) {
  if (!['muted', 'unmuted', 'kicked', 'left', 'joined'].includes(item.kind)) return null;
  const fields = {
    actor: item.actor ? who(item.actor) : '',
    target: item.target ? who(item.target) : say('records/someone'),
    span: span(item.seconds || 0),
  };
  const byOther = item.actor && item.actor !== item.target && ['muted', 'unmuted', 'kicked'].includes(item.kind);
  const line = element('div', 'records-event');
  line.append(element('span', 'muted', when(item.at)), ` ${say(`records/event-${item.kind}${byOther ? '' : '-plain'}`, fields)}`);
  return line;
}

/** 禁言多久：秒、分钟、小时、天里最大的那个整数。 */
function span(seconds) {
  for (const [unit, size] of [['days', 86400], ['hours', 3600], ['minutes', 60]]) {
    if (seconds >= size && seconds % size === 0) return say(`records/${unit}`, { count: seconds / size });
  }
  if (seconds >= 60) return say('records/minutes', { count: Math.round(seconds / 60) });
  return say('records/seconds', { count: seconds });
}

/** 带的一样东西的占位：`[图片]`、`[文件 排班.pdf]`…… */
export function placeholder(one) {
  const kind = ['image', 'file', 'voice', 'video', 'sticker'].includes(one.kind) ? one.kind : 'file';
  if (kind === 'file' && one.name) return say('records/media-file-named', { name: one.name });
  return say(`records/media-${kind}`);
}

/** 一条只带东西没写字的，几样占位连起来。 */
function placeholders(item) {
  return (item.media || []).map(placeholder).join(' ');
}

/** 引用里的字最多四十个。 */
function clip(text) {
  const chars = Array.from(text);
  return chars.length > 40 ? `${chars.slice(0, 40).join('')}…` : text;
}
