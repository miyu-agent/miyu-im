// 会话卡上的小标签（施工 I-7 上，设计 01 第八节第 3 条「几个说明它怎么设的小标签」）：只说卡上写了的（和默认不一样的），一样都没写
// 的不写（I-7 上修，项目主人 10-20：去掉「照默认」）；关着的先写「只记录」。最多 MOST 个，多的写「+N」。
import { say } from '../../shell/i18n.js';

const MOST = 4;

/** `one`（model.js 的一张卡）的标签：[{ text, accent }]；`personas` 是人格编号到名字。 */
export function chips(one, personas) {
  const set = one.set;
  const all = [];
  if (one.on === false) all.push({ text: say('chip/off') });
  // 线路规程、抽样（施工 I-7 中二）：写在卡上 `rules` 里的照它，没写的照插嘴的旧写法（四档）。
  const rules = set.rules || {};
  if (rules.discipline === 'when-called') all.push({ text: say('chip/called-only') });
  else if (rules['chatty.probability'] !== undefined) {
    all.push({ text: say('chip/chatty-percent', { n: rules['chatty.probability'] / 10 }), accent: true });
  } else if (set.chatty) {
    all.push({ text: set.chatty === 'never' ? say('chip/called-only') : say(`chip/chatty-${set.chatty}`), accent: set.chatty !== 'never' });
  }
  if (set.keywords?.length) all.push({ text: say('chip/keywords', { words: set.keywords.join(say('chip/joiner')) }) });
  if (set.persona) all.push({ text: say('chip/persona', { name: personas.get(set.persona) || set.persona }) });
  if (set.model) all.push({ text: say('chip/model', { model: set.model }) });
  if (set.extra_prompt) all.push({ text: say('chip/extra-prompt') });
  if (set.rate) all.push({ text: say('chip/rate', { rate: set.rate }) });
  if (set.sleep) all.push({ text: say('chip/sleep') });
  if (set.managers?.length) all.push({ text: say('chip/managers', { count: set.managers.length }) });
  if (set.show_ids !== undefined) all.push({ text: say(set.show_ids ? 'chip/ids-shown' : 'chip/ids-hidden') });
  if (set.recent) all.push({ text: say('chip/recent', { count: set.recent }) });
  if (set.long_text) all.push({ text: say(`chip/long-${set.long_text}`) });
  if (set.final_only !== undefined) all.push({ text: say(set.final_only ? 'chip/final-only' : 'chip/every-step') });
  if (all.length <= MOST) return all;
  return [...all.slice(0, MOST - 1), { text: `+${all.length - MOST + 1}` }];
}
