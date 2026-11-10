// 会话浮窗各页的格（施工 I-7 中一，名字、小字、默认照 docs/designs/01-接入通讯平台-配置项.md）：每一格写它是卡上的哪一格、什么控件、
// 哪几种卡有、要哪一格开着才画。form.js 照它画、存。
//
// - key：卡上的格；`rules.` 开头的是卡上 `rules` 里的规则键（宿主 card_edit 照项部分更新）。
// - control：select（下拉，`options` 交选项）、persona、preset（人格、预设的下拉，选项照宿主给的）、model（点开小窗选模型）、text、area、switch（`values` 是开、
//   关存成什么，不写是真假）、number（带 `unit`）、float（小数）、multi（几个小数一行，`keys` 各存各的）、seconds（规则里写成 `15s`
//   的时长，页面上填秒数）、rate（`N/Ms`：几条 / 几秒）、sleep（`HH:MM-HH:MM`）、tags（字的列表；`people` 的照号写、认得的写名字）、
//   reaction（贴表情：开关加点开小窗选）。`prefixed` 的标签只填了号的补成平台上的人的编号（`qq:10002`）。
// - kinds：哪几种卡有（group、other_groups、private、other_privates），不写是都有。
// - dep：`{ key, on, unset? }`：那一格的值是 `on` 时才画（开关管着的几行关着时藏起来，设计第八节第 3 条）；那一格没写的照 `unset`，
//   不写 `unset` 的当是 `on`。switch 也认 `unset`：没写、也没继承到的照它画。
// - advanced：收进「高级」里（默认收起）。
// - empty：没有继承的值时输入框的占位字（施工 I-168：使用限制空着是「不限」）。
// - warn：`(one, context) => 字`，这一格下面先写的一句（施工 I-189：上下文上限设不上时的「最少 x」）；空的不写。
// - needs：浮窗问到的那一样（`reactions`：平台能贴的表情；`presets`：预设）是空的、问不到的就不画（照回答里同名的那个列表）。
// - optionCaps：下拉的哪一项要平台做得到哪一样（施工 I-85 补，连接的 `caps`）：做不到的不列，卡上已经写了它的照旧列；会话默认值
//   那张不分平台，都列。
import { say } from '../../shell/i18n.js';

const GROUPS = ['group', 'other_groups'];

/** 主动回复判断开着才画（线路规程是 `chatty`）。 */
const JUDGE = { key: 'rules.discipline', on: 'chatty' };

/** 五项权重：相关性、意愿、社交适合度、时机、连续性。 */
const WEIGHTS = ['relevance', 'willingness', 'social', 'timing', 'continuity'].map((name) => `rules.chatty.${name}`);
const ALL_BUT_PRIVATE = ['group', 'other_groups', 'other_privates'];

/** 使用限制的三格（`kinds` 那几种卡，`dep` 照给的）：第一格带小字。 */
function limits(kinds, dep) {
  return ['5h', 'week', 'month'].map((line, index) => ({
    key: `rules.limit_${line}`, control: 'number', label: `field/limit-${line}`, hint: index === 0 ? 'field/limit-5h-hint' : undefined,
    unit: 'unit/tokens', empty: 'limits-page/unlimited', kinds, dep, min: 1, max: 1000000000000,
  }));
}

/** 上下文上限设不上的（施工 I-189）：宿主问卡片时交的 `context_min` 里有这张卡的（群卡、私聊卡照号），写最少多少。 */
function contextWarn(one, context) {
  const kind = one.kind === 'group' || one.kind === 'private' ? one.kind : null;
  const min = kind ? context.contextMin?.[kind]?.[String(one.id)] : undefined;
  return min ? say('field/context-limit-min', { min: Number(min).toLocaleString() }) : '';
}

/** 各页的格，照页的编号。 */
export const FIELDS = {
  persona: [
    { key: 'persona', control: 'persona', label: 'field/persona' },
    { key: 'preset', control: 'preset', label: 'field/preset', needs: 'presets' },
    { key: 'extra_prompt', control: 'area', label: 'field/extra-prompt', hint: 'field/extra-prompt-hint', defaultsHint: 'field/extra-prompt-hint-defaults', wide: true },
    { group: 'field/models' },
    { key: 'model', control: 'model', label: 'field/model', use: 'chat' },
    // 会话的上下文上限（施工 I-189，核心 CTX-1）：token 数，空着照模型的窗口；设得太小的核心不收，下面写最少多少。
    { key: 'rules.context', control: 'number', label: 'field/context-limit', hint: 'field/context-limit-hint', unit: 'unit/tokens', empty: 'field/context-limit-empty', min: 1, warn: contextWarn },
    // 判官没配模型的走旁路请求 `judge`（施工 I-42）：落到池的写池，池空了的照这张卡的模型（`orKey`），再照核心的对话模型。
    { key: 'rules.judge.model', control: 'model', label: 'field/judge-model', use: 'judge', orKey: 'model', kinds: GROUPS, dep: JUDGE },
    { group: 'field/context' },
    { key: 'recent', control: 'number', label: 'field/recent', unit: 'unit/messages', kinds: GROUPS, min: 1, max: 1000 },
    // 是否向 AI 显示号码（2026-10-20 项目主人验收：每种卡都有，原来只在群卡的触发方式页）。
    { key: 'show_ids', control: 'switch', label: 'field/show-ids' },
  ],
  // 分组（设计 11 G1，施工 I-165）：什么时候说话、主动回复判断、权限；「高级」的照旧收进折叠。
  trigger: [
    { group: 'field/when-to-speak' },
    { key: 'keywords', control: 'tags', label: 'field/keywords', hint: 'field/keywords-hint', kinds: GROUPS },
    // 时不时看一眼（设计 05，施工 I-89）：名单里的群卡、私聊卡（I-89 下）、会话默认值（`has` 把默认值当群卡），两张「其他」没有。
    { key: 'glance', control: 'switch', label: 'field/glance', hint: 'field/glance-hint', kinds: ['group', 'private'] },
    // 她没定下一次时多久看一眼（施工 I-114）：她用工具定了的照她的。
    { key: 'glance_minutes', control: 'number', label: 'field/glance-minutes', hint: 'field/glance-minutes-hint', unit: 'unit/minutes', kinds: ['group', 'private'], min: 1, max: 10080, dep: { key: 'glance', on: true } },
    // 看一眼时的额外提示词（施工 I-196，设计 12 第 2 条）：这一轮要做的事，和人格页的额外提示词（改人格的）分开。
    { key: 'rules.glance_note', control: 'area', label: 'field/glance-note', hint: 'field/glance-note-hint', kinds: ['group', 'private'], wide: true, dep: { key: 'glance', on: true } },
    // 看一眼那一轮最多交几条没回的新消息（施工 I-175，项目主人 11-05）：只有群，私聊到点不看新消息。没写的照桥的出厂（50）。
    { key: 'glance_records', control: 'number', label: 'field/glance-records', hint: 'field/glance-records-hint', unit: 'unit/messages', kinds: ['group'], min: 1, max: 64, dep: { key: 'glance', on: true } },
    { group: 'field/judging' },
    // 概率触发主动回复（施工 I-197，设计 12 第一节第 1 条）：和主动回复判断是两个开关，都开着的抽中的再过判断。
    { key: 'rules.chatty.sample', control: 'switch', label: 'field/sample', hint: 'field/sample-hint', kinds: GROUPS },
    { key: 'rules.chatty.sample_rate', control: 'number', scale: 10, label: 'field/sample-rate', unit: 'unit/percent', kinds: GROUPS, min: 0, max: 1000, dep: { key: 'rules.chatty.sample', on: true } },
    { key: 'rules.chatty.sample_skip_media', control: 'switch', label: 'field/sample-skip-media', hint: 'field/sample-skip-media-hint', kinds: GROUPS, dep: { key: 'rules.chatty.sample', on: true } },
    { key: 'rules.discipline', control: 'switch', values: ['chatty', 'when-called'], label: 'field/judge', hint: 'field/judge-hint', kinds: GROUPS },
    { key: 'rules.chatty.judge_called', control: 'switch', label: 'field/judge-called', hint: 'field/judge-called-hint', kinds: GROUPS, dep: JUDGE },
    { key: 'rules.judge.records', control: 'number', label: 'field/judge-records', unit: 'unit/messages', kinds: GROUPS, dep: JUDGE, min: 1, max: 100 },
    { key: 'rules.judge.persona', control: 'switch', label: 'field/judge-persona', kinds: GROUPS, dep: JUDGE },
    { key: 'rules.chatty.skip_users', control: 'tags', people: true, prefixed: true, label: 'field/skip-users', hint: 'field/skip-users-hint', kinds: GROUPS, dep: JUDGE },
    { group: 'field/permissions' },
    { key: 'managers', control: 'tags', people: true, label: 'field/managers', kinds: GROUPS },
    { key: 'rules.chatty.base', control: 'float', label: 'field/threshold', kinds: GROUPS, dep: JUDGE, advanced: true },
    { key: 'rules.chatty.relevance', control: 'multi', keys: WEIGHTS, label: 'field/weights', kinds: GROUPS, dep: JUDGE, advanced: true },
    { key: 'rules.chatty.restraint', control: 'switch', label: 'field/restraint', hint: 'field/restraint-hint', kinds: GROUPS, dep: JUDGE, advanced: true },
    // 跟进（施工 I-21）：她正在为这个人跑一轮时他又说了一条，过了判断并进那一轮。
    { key: 'rules.chatty.followup', control: 'switch', label: 'field/followup', hint: 'field/followup-hint', kinds: GROUPS, dep: JUDGE, advanced: true },
    { key: 'rules.chatty.continuation_window', control: 'seconds', label: 'field/continuation', hint: 'field/continuation-hint', kinds: GROUPS, dep: JUDGE, advanced: true },
    { key: 'rules.chatty.after_speaking_window', control: 'seconds', label: 'field/after-speaking', hint: 'field/after-speaking-hint', kinds: GROUPS, dep: JUDGE, advanced: true },
    { key: 'rules.dispatch.supersede_window', control: 'seconds', label: 'field/supersede', hint: 'field/supersede-hint', kinds: GROUPS, dep: JUDGE, advanced: true },
    // 违规判断的门槛（施工 I-16，照旧版设置页）：违规关键词在设置页「违规判断」。
    { key: 'rules.chatty.severity_min', control: 'number', label: 'field/severity-min', hint: 'field/severity-min-hint', kinds: GROUPS, advanced: true, min: 0, max: 10 },
  ],
  // 分组（施工 I-165）：限流和睡眠、使用限制、并发。
  pace: [
    { group: 'field/rate-sleep' },
    // 私聊白名单里的人（施工 I-7 中三下一）：忽略限流、忽略睡眠没写的是忽略（`unset`），关了才画限流、睡眠，写在卡上的 `rules` 里。
    { key: 'rules.ignore_rate', control: 'switch', unset: true, label: 'field/ignore-rate', kinds: ['private'] },
    { key: 'rules.rate', control: 'rate', label: 'field/rate', hint: 'field/rate-hint', kinds: ['private'], dep: { key: 'rules.ignore_rate', on: false, unset: true } },
    { key: 'rules.ignore_sleep', control: 'switch', unset: true, label: 'field/ignore-sleep', hint: 'field/ignore-sleep-hint', kinds: ['private'] },
    { key: 'rules.sleep', control: 'sleep', label: 'field/sleep', hint: 'field/sleep-hint', kinds: ['private'], dep: { key: 'rules.ignore_sleep', on: false, unset: true } },
    { key: 'rate', control: 'rate', label: 'field/rate', hint: 'field/rate-hint', kinds: ALL_BUT_PRIVATE },
    { key: 'sleep', control: 'sleep', label: 'field/sleep', hint: 'field/sleep-hint', kinds: ALL_BUT_PRIVATE },
    { group: 'field/limits' },
    // 使用限制（设计 08，施工 I-135）：这个会话最近 5 小时、7 天、30 天最多用多少 token，没写的不限。私聊卡照限流，关了忽略限流才画。
    ...limits(ALL_BUT_PRIVATE),
    ...limits(['private'], { key: 'rules.ignore_rate', on: false, unset: true }),
    { group: 'field/concurrency' },
    { key: 'rules.parallel', control: 'number', label: 'field/parallel', hint: 'field/parallel-hint', unit: 'unit/people', kinds: GROUPS, min: 0, max: 16 },
  ],
  // 分组（施工 I-165）：说话方式、怎么发、长文。
  send: [
    { group: 'field/speaking' },
    // 这个会话的说话方式（施工 I-140，I-141 改说法：写这里的人怎么说话，不写她是谁）：写了的，群里每开一轮都带上这段；不写不带。
    { key: 'rules.style_note', control: 'area', label: 'field/style-note', hint: 'field/style-note-hint', defaultsHint: 'field/style-note-defaults', kinds: GROUPS, wide: true },
    // 去掉末尾句号、一条分多条发送、打字速度（施工 I-49，项目主人 10-27：每张卡片单独设，原来在设置页「通用」里管所有会话；施工 I-165 起
    // 句号在「说话方式」、分条和速度在「怎么发」）。
    { key: 'rules.outbound.strip_period', control: 'switch', label: 'field/strip-period', hint: 'field/strip-period-hint' },
    // 智能引用、智能艾特、贴表情（施工 I-7 中三下二）。贴表情要平台给了能贴的表情（`needs`）。
    { key: 'rules.outbound.quote', control: 'switch', label: 'field/quote', hint: 'field/quote-hint' },
    { key: 'rules.outbound.quote_after', control: 'number', label: 'field/quote-after', unit: 'unit/messages', advanced: true, min: 0, max: 1000, dep: { key: 'rules.outbound.quote', on: true } },
    { key: 'rules.outbound.mention', control: 'switch', label: 'field/mention', hint: 'field/mention-hint', kinds: GROUPS },
    { key: 'rules.outbound.mention_after', control: 'seconds', label: 'field/mention-after', kinds: GROUPS, advanced: true, min: 1, max: 86400, dep: { key: 'rules.outbound.mention', on: true } },
    { key: 'rules.reaction', control: 'reaction', label: 'field/reaction', hint: 'field/reaction-hint', kinds: GROUPS, needs: 'reactions' },
    { group: 'field/sending' },
    { key: 'rules.outbound.humanlike', control: 'switch', label: 'field/humanlike', hint: 'field/humanlike-hint' },
    // 分条和停顿是两个开关（施工 I-195，设计 12 第 4 条）：不分条的时候停顿、打字速度都不出现；不停顿的时候打字速度不出现。
    { key: 'rules.outbound.pause', control: 'switch', label: 'field/pause', hint: 'field/pause-hint', dep: { key: 'rules.outbound.humanlike', on: true } },
    { key: 'rules.outbound.typing_speed', control: 'number', label: 'field/typing-speed', hint: 'field/typing-speed-hint', unit: 'unit/chars-per-minute', min: 20, max: 600, dep: [{ key: 'rules.outbound.humanlike', on: true }, { key: 'rules.outbound.pause', on: true }] },
    // 按顺序发送（施工 I-159，项目主人 11-05：关了全部立即发）。
    { key: 'rules.outbound.ordered', control: 'switch', label: 'field/ordered', hint: 'field/ordered-hint' },
    { key: 'final_only', control: 'switch', label: 'field/final-only', hint: 'field/final-only-hint' },
    { group: 'field/long' },
    { key: 'long_text', control: 'select', label: 'field/long-text', hint: 'field/long-text-hint', options: ['image', 'forward'], optionCaps: { forward: 'forward' }, optionText: 'long/' },
    { key: 'long_chars', control: 'number', label: 'field/long-chars', hint: 'field/long-chars-hint', unit: 'unit/chars', min: 0, max: 100000 },
  ],
};

/** `one`（model.js 的一张卡）上 `field` 有没有。会话默认值（施工 I-7 下二，设置页）照群卡：群卡有的都有，私聊卡才有的没有。 */
export function has(field, one) {
  // 群聊、私聊默认值照群卡、私聊卡的格画（施工 I-112）。
  const kind = { defaults: 'group', private_defaults: 'private' }[one.kind] || one.kind;
  return !field.kinds || field.kinds.includes(kind);
}

/** 没写这一格时跟着谁的说法（施工 I-111，项目主人 11-02：「跟随默认」有歧义）：群聊、私聊默认值那两张跟着全局（核心的配置），
 *  群卡跟着群聊默认值，私聊卡跟着私聊默认值（施工 I-112）。`value` 是跟到的值，空的不写括号。 */
export function inherit(one, value) {
  // 施工 I-112 起两张默认值：群卡跟群聊的、私聊卡跟私聊的。
  const key = one.kind === 'defaults' || one.kind === 'private_defaults'
    ? 'field/inherit-global'
    : one.type === 'private'
      ? 'field/inherit-private'
      : 'field/inherit-group';
  return value == null ? say(`${key}-bare`) : say(key, { value });
}

/** 卡上写着的值：没写的是 undefined。 */
export function written(one, key) {
  if (key.startsWith('rules.')) return one.set.rules?.[key.slice(6)];
  return one.set[key];
}

/** 卡下面几层的值和来处 `{value, from}`：卡片默认值里没有的是空的。 */
export function inherited(one, key) {
  if (key.startsWith('rules.')) return one.defaults.rules?.[key.slice(6)] || { value: null, from: null };
  return one.defaults[key] || { value: null, from: null };
}

/** 现在用着的：卡上写着的，没写的照下面几层。 */
export function current(one, key) {
  const value = written(one, key);
  if (value !== undefined) return value;
  return inherited(one, key).value;
}

/** 存 `key` 那一格成 `value`（null 是恢复默认）时交给 cards.save_one 的 `values`。 */
export function values(key, value) {
  if (key.startsWith('rules.')) return { rules: { [key.slice(6)]: value } };
  return { [key]: value };
}
