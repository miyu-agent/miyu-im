// 会话页的数据（施工 I-7 上）：宿主的 cards.get 交来卡片、每一格的默认值、私聊白名单、AI 在的群和好友（带问到它的机器人）、人格，
// 这里拼成墙上一张一张的卡。
//
// - 先「会话默认值」（`cards.json` 的 `defaults`，2026-10-20 从设置页挪过来，kind 是 defaults、照群卡的格画），再「陌生群」「陌生私聊」
//   两张兜底卡，再群聊名单（卡片里的群），再私聊白名单（白名单里的人，加上有卡的；卡是存第一格时才建的）。
// - 是否回复（on）：卡上写了的照它；没写的群卡回复，「陌生群」「陌生私聊」不回复（这一格卡片一定写进规则，不看下面几层）。私聊白名单
//   里的人没有这一格（在名单里就是要回）。
// - 名字照 AI 在的群、好友的表，表里没有的写号。

/** 一张卡：kind（defaults、other_groups、other_privates、group、private）、type（group、private：筛选用）、id（号，兜底卡没有）、
 *  name、members、bot（问到它的机器人的号）、set（卡上写了的格）、defaults（每一格的 {value, from}）、on（是否回复，没有这一格的
 *  是 null）。 */
export function entries(answer) {
  const cards = answer.cards || {};
  const defaults = answer.defaults || {};
  const groups = new Map((answer.groups || []).map((one) => [String(one.id), one]));
  const friends = new Map((answer.friends || []).map((one) => [String(one.id), one]));
  const all = [
    entry('defaults', 'group', null, cards.defaults, defaults.defaults),
    entry('other_groups', 'group', null, cards.other_groups, defaults.other_groups),
    entry('other_privates', 'private', null, cards.other_privates, defaults.other_privates),
  ];
  for (const card of cards.groups || []) {
    const id = String(card.id);
    const known = groups.get(id);
    all.push({
      ...entry('group', 'group', id, card.set, defaults.groups?.[id]),
      name: known?.name || id,
      members: known?.members ?? null,
      bot: known?.bot ?? null,
    });
  }
  const sets = new Map((cards.privates || []).map((card) => [String(card.id), card.set]));
  const people = [...new Set([...(answer.whitelist || []).map(String), ...sets.keys()])];
  for (const id of people) {
    const known = friends.get(id);
    all.push({
      ...entry('private', 'private', id, sets.get(id), defaults.privates?.[id]),
      name: known?.name || id,
      bot: known?.bot ?? null,
    });
  }
  return all;
}

function entry(kind, type, id, set, defaults) {
  const card = set || {};
  const shown = defaults || {};
  const on = kind === 'private' || kind === 'defaults' ? null : card.on ?? kind === 'group';
  return { kind, type, id, name: '', members: null, bot: null, set: card, defaults: shown, on };
}

/** 页面交给 cards.save_one 的那张卡。 */
export function cardOf(one) {
  // 号照原样交（施工 I-85 中：别的平台的号是字、负数；宿主数、字都认）。
  return one.id ? { kind: one.kind, id: one.id } : { kind: one.kind };
}

/** 把 `one` 的 `key` 那一格改成 `value`：和默认一样的交 null（恢复默认，卡上只写和默认不一样的）。是否回复的默认见上；别的照卡
 *  下面几层。 */
export function valueFor(one, key, value) {
  const fallback = key === 'on' ? one.kind === 'group' : one.defaults[key]?.value;
  return fallback !== undefined && JSON.stringify(fallback) === JSON.stringify(value) ? null : value;
}
