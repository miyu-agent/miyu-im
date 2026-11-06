// 数据统计的两张排行（设计 11 第三节第 3 条，施工 I-169）：按人（这一段谁用了多少 token，im 自己的个人账本）、按会话（token、花费、
// 收到、回复并成一张，原来「按场所」「最热闹」两张小表）。都是共用的列表（shell/listing.js）：能搜、能换排序，20 条以后「再显示」；
// 搜了什么、怎么排记在 `state` 里，重画时照旧。
import { element, say } from '../../shell/i18n.js';
import { listing } from '../../shell/listing.js';
import { sect } from '../../shell/sect.js';
import { money, short, venueLink } from './stats.js';

/** 按人：`people` 是 stats 回答里的 [{id, name, tokens}]。 */
export function peopleRank(people, state) {
  const card = sect(say('stats/by-person'), say('stats/by-person-hint'));
  if (people.length === 0) {
    card.body.append(element('p', 'faint', say('stats/none')));
    return card.box;
  }
  const items = people.map((one) => {
    const label = one.name ? `${one.name} · ${one.id}` : one.id;
    return { node: line([label], [short(one.tokens)]), text: label, numbers: { tokens: one.tokens } };
  });
  card.body.append(listing(items, { search: say('listing/search'), state, header: head([say('stats/col-person'), say('stats/col-tokens')]) }));
  return card.box;
}

/** 按会话：`token.venues`（token、花费）和 `messages.venues`（收到、回复）照场所并起来。`context` 照 stats.js 的（会话名、连接）。 */
export function venueRank(shown, context, state) {
  const card = sect(say('stats/by-venue'));
  const merged = new Map();
  const of = (name) => {
    if (!merged.has(name)) merged.set(name, { name, tokens: 0, cost: null, received: 0, replied: 0 });
    return merged.get(name);
  };
  for (const one of shown.token?.venues || []) Object.assign(of(one.name), { tokens: one.tokens, cost: one.cost ?? null });
  for (const one of shown.messages?.venues || []) Object.assign(of(one.name), { received: one.received, replied: one.replied ?? 0 });
  if (merged.size === 0) {
    card.body.append(element('p', 'faint', say('stats/none')));
    return card.box;
  }
  const currency = shown.token?.cost?.currency;
  const items = [...merged.values()].map((one) => {
    const link = venueLink(one.name, context);
    const node = line([link], [short(one.tokens), one.cost == null ? '—' : money(one.cost, currency), short(one.received), short(one.replied)]);
    return { node, text: link.textContent, numbers: { tokens: one.tokens, cost: one.cost ?? 0, received: one.received, replied: one.replied } };
  });
  const sorts = ['tokens', 'cost', 'received', 'replied'].map((key) => [key, say(`stats/sort-${key}`)]);
  const header = head([say('stats/col-venue'), say('stats/col-tokens'), say('stats/col-cost'), say('stats/col-received'), say('stats/col-replied')]);
  card.body.append(listing(items, { search: say('listing/search'), sorts, state, header }));
  return card.box;
}

/** 列名那一行。 */
function head(columns) {
  const row = element('div', `rank-row rank-head rank-${columns.length}`);
  columns.forEach((label, at) => row.append(element('span', at ? 'th num' : 'th', label)));
  return row;
}

/** 一行：第一格是名字（字或节点），后面的数靠右。 */
function line([name], numbers) {
  const row = element('div', `rank-row rank-${numbers.length + 1}`);
  const shown = typeof name === 'string' ? element('span', null, name) : name;
  shown.classList.add('name');
  if (typeof name === 'string') shown.title = name;
  row.append(shown, ...numbers.map((value) => element('span', 'num', value)));
  return row;
}
