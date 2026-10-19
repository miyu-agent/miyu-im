// 会话卡、浮窗上的名字和号那一行（施工 I-7 上）。号的叫法由平台给（设计 01 第八节第 7 条：界面上不写死「QQ 号」「群号」，平台没给的
// 写「群 ID」「用户 ID」）；同一个平台连着不止一个机器人时，后面写是哪个机器人。
import { say } from '../../shell/i18n.js';
import { platform } from '../home/platforms.js';

/** 卡的名字：兜底卡是「陌生群」「陌生私聊」。 */
export function titleOf(one) {
  if (one.kind === 'other_groups') return say('venues/other-groups');
  if (one.kind === 'other_privates') return say('venues/other-privates');
  return one.name;
}

/** 名字下面那一行：`context` 是 { platform, bots }。 */
export function idLine(one, context) {
  if (one.kind === 'other_groups') return say('venues/other-groups-hint');
  if (one.kind === 'other_privates') return say('venues/other-privates-hint');
  const parts = [`${idWord(one.type, context.platform)} ${one.id}`];
  if (one.members) parts.push(say('venues/members', { count: one.members }));
  if (context.bots.size > 1 && one.bot) parts.push(context.bots.get(String(one.bot)) || String(one.bot));
  return parts.join(' · ');
}

/** 平台 `id` 上群号、用户号的叫法。 */
export function idWord(type, id) {
  const shown = id ? platform(id) : null;
  if (type === 'group') return shown?.groupId?.() || say('venues/group-id');
  return shown?.userId?.() || say('venues/user-id');
}
