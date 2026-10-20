// 首页日志的「活动」（施工 I-16 / I-19，2026-10-20 项目主人：旧版的日志简陋但可读，新版现在看不出什么）：运行日志里几种事照中文
// 一件一行写出来——谁在哪说了什么、回没回、为什么（分数、门槛、判官的理由）；AI 回了什么；调了什么工具、结果；没发出去的、撤回、
// 补拉、排队。别的提醒、出错照原样写。数据是宿主的运行日志（`logs`），这些行宿主多记了 who、text、why、said 这几格。
import { element, say } from '../../shell/i18n.js';
import { reason } from '../venues/overview.js';

/** 一行日志的字拆成「这件事」和几格（值去掉引号、解开转义）。 */
export function fieldsOf(text) {
  const at = text.search(/ [a-z_][a-z0-9_.]*=/);
  const msg = at < 0 ? text : text.slice(0, at);
  const fields = {};
  if (at >= 0) {
    for (const field of text.slice(at).matchAll(/\s*([a-z_][a-z0-9_.]*)=("(?:[^"\\]|\\.)*"|\S*)/g)) {
      let value = field[2];
      if (value.startsWith('"')) {
        try {
          value = JSON.parse(value);
        } catch {
          value = value.slice(1, -1);
        }
      }
      fields[field[1]] = value;
    }
  }
  return { msg: msg.trim(), fields };
}

/** 场所（`qq:group:777`）照名字写：会话页的名单里有名字的写名字，没有的写「群 777」。 */
function place(venue, names) {
  if (!venue) return '';
  const [, kind, number] = String(venue).split(':');
  return names.get(`${kind}:${number}`) || say(`stats/kind-${kind}`) + ` ${number || ''}`;
}

/** 判断说成中文：回没回、为什么（同会话浮窗「最近判断」的说法，判过的带分数和门槛）。 */
function decided(fields) {
  const outcome = say(`activity/outcome-${fields.outcome}`);
  const number = (text) => (text === undefined || text === '' ? undefined : Number(text));
  const why = { reason: fields.why || 'quiet', trigger: fields.trigger || undefined, total: number(fields.total), threshold: number(fields.threshold) };
  return fields.why ? `${outcome} · ${reason({ outcome: fields.outcome, why })}` : outcome;
}

/** 一行日志照活动写成一条：认得的几种照中文，别的提醒、出错照原样；平常的别的不写（空的）。 */
function described(line, names) {
  const { msg, fields } = fieldsOf(line.text);
  const where = place(fields.venue, names);
  const tool = fields.tool ? say(`activity/tool-${fields.tool}`) : '';
  const toolName = tool === `activity/tool-${fields.tool}` ? fields.tool : tool;
  switch (msg) {
    case 'chat decided':
      return {
        // 以前的日志行没有谁、说了什么（施工 I-16 以前写的）：说成「一条消息」。
        head: fields.who || fields.text
          ? say('activity/said', { where, who: fields.who || '', text: fields.text || '' })
          : say('activity/a-message', { where }),
        tail: decided(fields),
        note: fields.judge || '',
        tone: fields.outcome === 'reply' ? 'good' : null,
      };
    case 'reply sent':
      return { head: say('activity/replied', { where, text: fields.text || '' }), tone: 'ai' };
    case 'reply dropped':
      return { head: say('activity/dropped', { where, why: fields.why || '' }), tone: 'warn' };
    case 'platform tool done':
      return { head: say('activity/tool', { where, tool: toolName, said: fields.said || '' }) };
    case 'platform tool failed':
      return { head: say('activity/tool-failed', { where, tool: toolName, said: fields.said || '' }), tone: 'warn' };
    case 'message backfilled':
      return { head: say('activity/backfilled', { where }) };
    case 'recall noted':
      return { head: say('activity/recalled', { where }) };
    case 'reply queued':
      return { head: say('activity/queued', { where }) };
    case 'reply queue full':
      return { head: say('activity/queue-full', { where }), tone: 'warn' };
    case 'turn interrupted after recall':
      return { head: say('activity/interrupted', { where }) };
    case 'long text not drawn':
      return { head: say('activity/not-drawn', { where, why: fields.why || '' }), tone: 'warn' };
    default:
      if (line.level === 'WARN' || line.level === 'ERROR') return { head: line.text, tone: 'warn' };
      return null;
  }
}

/** 活动的几行：新的在前。 */
export function activityRows(logs, names) {
  const rows = [];
  for (const line of logs) {
    const one = described(line, names);
    if (!one) continue;
    const row = element('div', one.tone ? `act act-${one.tone}` : 'act');
    const time = element('span', 'faint act-time', line.time);
    time.title = line.date;
    const body = element('div', 'act-body');
    body.append(element('div', 'act-head', one.head));
    if (one.tail) body.append(element('div', 'act-tail faint', one.tail));
    if (one.note) body.append(element('div', 'act-note faint', one.note));
    row.append(time, body);
    row.title = line.text;
    rows.push(row);
  }
  return rows;
}

/** 会话页名单里的名字：`group:777` → 群名，`private:10001` → 好友名。 */
export function namesOf(cards) {
  const names = new Map();
  for (const group of cards?.groups || []) if (group.name) names.set(`group:${group.id}`, group.name);
  for (const friend of cards?.friends || []) if (friend.name) names.set(`private:${friend.id}`, friend.name);
  return names;
}
