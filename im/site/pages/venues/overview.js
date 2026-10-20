// 会话浮窗「概况」里的今天和最近判断（施工 I-7 上补，设计 01 第八节第 7 条，照样板 d-im.html 的 .facts、.why）：宿主的
// cards.overview 交这张卡今天的数和最近判断（新的在前），这里画。
//
// - 三个数：今天收到、今天回复、只记录。
// - 最近判断（只有群聊的：私聊不判）：一条一行，回了的打勾、没回的一横；引号里是那条消息的开头，下面「谁 · 几点 · 为什么」。为什么照
//   宿主给的原因编号写（noted.rs 的那几种，`skip_user` 是在跳过判断的人里），挂接点改了结论的后面写是哪个插件；带了判官的理由
//   （`why.reason_text`）的再写一行小字。
import { element, say } from '../../shell/i18n.js';

/** 画进 `holder`：`answer` 是 cards.overview 的回答，`type` 是 group、private。 */
export function drawOverview(holder, answer, type) {
  const facts = element('div', 'facts');
  for (const [key, label] of [
    ['received', 'venue/today-received'],
    ['replied', 'venue/today-replied'],
    ['recorded', 'venue/today-recorded'],
  ]) {
    const fact = element('div', 'fact');
    fact.append(element('b', null, Number(answer.today?.[key] ?? 0).toLocaleString()), element('span', null, say(label)));
    facts.append(fact);
  }
  const parts = [facts];
  if (type === 'group') {
    parts.push(element('div', 'group-name', say('venue/recent')));
    const recent = answer.recent || [];
    if (recent.length === 0) parts.push(element('p', 'faint', say('venue/recent-none')));
    for (const note of recent) parts.push(row(note));
  }
  holder.replaceChildren(...parts);
}

/** 一笔判断。 */
function row(note) {
  const replied = note.outcome !== 'record';
  const line = element('div', 'why');
  const mark = element('span', replied ? 'mark yes' : 'mark no', replied ? '✓' : '–');
  const grow = element('div', 'grow');
  const time = new Date(note.at).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  grow.append(element('q', null, note.text || say('venue/no-text')), element('small', null, [note.who, time, reason(note)].join(' · ')));
  // 判官说的理由（宿主给了 `why.reason_text` 的）：另起一行小字。
  const said = typeof note.why?.reason_text === 'string' ? note.why.reason_text.trim() : '';
  if (said) grow.append(element('p', 'why-said', said));
  line.append(mark, grow);
  return line;
}

/** 为什么那一句。 */
export function reason(note) {
  const why = note.why || {};
  const trigger = say(`trigger/${why.trigger || 'none'}`);
  const number = (value) => (typeof value === 'number' ? value.toFixed(2) : '—');
  let text;
  if (note.outcome === 'deferred') text = say('why/deferred');
  else if (why.reason === 'judged') {
    const fields = { trigger, total: number(why.total), threshold: number(why.threshold) };
    text = say(note.outcome === 'reply' ? 'why/judged-yes' : 'why/judged-no', fields);
  } else if (why.reason === 'unjudged') {
    text = say(note.outcome === 'reply' ? 'why/unjudged-reply' : 'why/unjudged-record', { trigger });
  } else text = say(`why/${why.reason || 'quiet'}`);
  return why.hook ? `${text}${say('why/hook', { package: why.hook })}` : text;
}
