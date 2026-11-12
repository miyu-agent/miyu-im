// 看一个人格的记忆（施工 I-207，项目主人 2026-11-10：im 网页也要能看人格的记忆，不然得去主网页）：只看，要改、要删去主网页。
//
// - 经转发口问核心的 memory.list（这个人格那一间，最新的在前，最多 500 条）和 memory.cards（认识的人的卡）。
// - 一张「记住的事」：一条一行，下面小字写哪一类、什么时候、在哪个群、说的是谁；能搜、能照类别筛（shell/listing.js）。
//   一张「认识的人」：一人一张卡，写怎么称呼、她对这个人的印象。
// - 没装记忆的、读不出的照核心给人看的那一句写。
import { element, say } from '../../shell/i18n.js';
import { openDialog } from '../../shell/dialog.js';
import { core, refusalText } from '../../shell/core.js';
import { listing } from '../../shell/listing.js';
import { sect } from '../../shell/sect.js';

/** 记忆的几类，照核心的 `class`。 */
const CLASSES = ['user', 'feedback', 'episode', 'reference', 'person'];

/** 一个时刻写成「月-日 时:分」。 */
function clock(text) {
  const at = new Date(text);
  if (Number.isNaN(at.getTime())) return '';
  return `${at.toLocaleDateString([], { year: 'numeric', month: '2-digit', day: '2-digit' })} ${at.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}`;
}

/** 一条记忆下面那行小字：哪一类、什么时候、在哪个群、说的是谁（纯逻辑，交回几段字）。 */
export function metaOf(memory) {
  const parts = [say(`memory/class-${memory.class}`), clock(memory.at)];
  if (memory.place) parts.push(memory.place);
  if (memory.who) parts.push(memory.who);
  else if ((memory.people || []).length) parts.push(memory.people.join('、'));
  return parts.filter(Boolean);
}

/** 开人格 `id`（名字 `name`）的记忆。 */
export function openMemory(request, id, name) {
  const dialog = openDialog(say('memory/title', { name }));
  dialog.body.classList.add('memory-body');
  dialog.body.append(element('p', 'faint', say('memory/loading')));
  const state = {};
  Promise.all([core(request, 'memory.list', { persona: id, limit: 500 }), core(request, 'memory.cards', { persona: id })]).then(
    ([listed, carded]) => {
      const memories = listed.memories || [];
      const cards = carded.cards || [];
      const facts = sect(say('memory/facts'), say('memory/readonly'));
      if (memories.length === 0) facts.body.append(element('p', 'faint', say('memory/none')));
      else {
        const items = memories.map((memory) => {
          const row = element('div', 'memory-row');
          row.append(element('div', 'memory-text', memory.text), element('small', null, metaOf(memory).join(' · ')));
          return { node: row, text: [memory.text, memory.who, memory.place, memory.keywords].filter(Boolean).join(' '), tags: { class: memory.class } };
        });
        facts.body.append(
          listing(items, {
            search: say('listing/search'),
            filters: [{ key: 'class', all: say('memory/class-all'), options: CLASSES.map((one) => [one, say(`memory/class-${one}`)]) }],
            state,
          }),
        );
      }
      const people = sect(say('memory/people'));
      if (cards.length === 0) people.body.append(element('p', 'faint', say('memory/no-people')));
      for (const card of cards) {
        const row = element('div', 'memory-row');
        const names = (card.names || []).length ? `${card.who}（${card.names.join('、')}）` : card.who;
        row.append(element('b', null, names), element('div', 'memory-text', card.text));
        people.body.append(row);
      }
      dialog.body.replaceChildren(facts.box, people.box);
    },
    (error) => dialog.body.replaceChildren(element('p', 'faint', refusalText(error))),
  );
}
