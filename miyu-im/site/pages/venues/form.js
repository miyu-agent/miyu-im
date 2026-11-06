// 会话浮窗一页的格（施工 I-7 中一）：照 fields.js 画一行一格，改了当场存（cards.save_one，和默认一样的交 null，卡上只写和默认不一样
// 的），存好了宿主推 cards、浮窗拿新的那一张重画。存不成的在那一行下面写一句。
//
// - 没写的格显示「继承（下面几层的值）」：下拉多一项、输入框写在占位字里、开关照现在用着的画。
// - 一组一张卡片（设计 11 G1，施工 I-165，shell/sect.js）：fields.js 的 `group` 起一张新卡、组名写在卡片第一行，第一个组前面的几格在一张没名字的
//   卡里；藏得一格不剩的卡不画。
// - 名字后面一个小标签（设计 11 G3，施工 I-165，shell/mark.js，换掉 I-111 开关下面那一行「跟随…」）：没写的「默认」，悬停说跟着谁（开关还说开、关）；
//   写了的「已改」，悬停换成「恢复默认」，点了这一格交 null。
// - 开关管着的几行（dep），开关关着时藏起来；「高级」的收进一个默认收起的折叠里。
// - 选人格、预设、模型的格旁边一个「去编辑」（设计 09，施工 I-152）：点了换到人格、预设、模型页，会话页收起时关掉这个浮窗。
import { element, icon, say, spaced } from '../../shell/i18n.js';
import { goTo } from '../../shell/connection.js';
import { toast } from '../../shell/dialog.js';
import { fieldMark } from '../../shell/mark.js';
import { filled, sect } from '../../shell/sect.js';
import { FIELDS, current, has, inherit, inherited, values, written } from './fields.js';
import { cardOf, isDefaults } from './model.js';
import { control } from './controls.js';

/** 选这几种的格旁边「去编辑」跳到哪一页。 */
const EDIT_PAGES = { persona: 'personas', preset: 'presets', model: 'models' };

/** 画 `tab` 那一页：`one` 是这张卡，`context` 是 { request, personas, models, reactions, presets, expanded }（expanded 记着哪几页的「高级」展开着，
 *  重画时照旧）。交回几块元素：几张卡片，有「高级」的最后是它的折叠。 */
export function drawForm(tab, one, context) {
  const sections = [];
  let section = sect(null);
  const advanced = [];
  for (const field of FIELDS[tab] || []) {
    if (!has(field, one)) continue;
    if (field.group) {
      sections.push(section);
      section = sect(say(field.group));
      continue;
    }
    if (field.dep && (current(one, field.dep.key) ?? field.dep.unset ?? field.dep.on) !== field.dep.on) continue;
    if (field.needs && !context[field.needs]?.[field.needs]?.length) continue;
    if (field.advanced) advanced.push(row(field, one, context));
    else section.body.append(row(field, one, context));
  }
  sections.push(section);
  const parts = filled(sections);
  if (advanced.length > 0) {
    const details = element('details', 'adv');
    details.open = context.expanded.has(tab);
    details.addEventListener('toggle', () => {
      if (details.open) context.expanded.add(tab);
      else context.expanded.delete(tab);
    });
    const inner = element('div', 'rows');
    inner.append(...advanced);
    details.append(element('summary', null, say('field/advanced')), inner);
    parts.push(details);
  }
  return parts;
}

/** 名字后面的小标签（shell/mark.js）：几项一行的（权重）任一项写了算「已改」，恢复时各项都交 null；开关没写的悬停还说开、关
 *  （施工 I-111，项目主人 11-02：改了会话默认值，没写的卡跟着变，卡上看不出来），别的控件自己写着继承的值。 */
function mark(field, one, saveKey) {
  const keys = field.keys || [field.key];
  const changed = keys.some((key) => written(one, key) !== undefined);
  const now = current(one, field.key) ?? field.unset;
  const [yes] = field.values || [true, false];
  const follows = field.control === 'switch' && now !== undefined && now !== null
    ? inherit(one, say(now === yes ? 'field/switch-on' : 'field/switch-off'))
    : inherit(one, null);
  return fieldMark(changed, follows, () => {
    for (const key of keys) saveKey(key, null);
  });
}

/** 一行：名字、小字、控件，下面一行写存不成的原因。 */
function row(field, one, context) {
  const line = element('div', field.wide ? 'row wide' : 'row');
  const label = element('div', 'label');
  const name = element('div', 'label-name', spaced(say(field.label, { id: context.idWord })));
  label.append(name);
  // 会话默认值那张卡上说「只在这个会话」不对的格另有一句（施工 I-7 下补）。
  const hint = (isDefaults(one) && field.defaultsHint) || field.hint;
  if (hint) label.append(element('small', null, say(hint)));
  const failed = element('div', 'row-failed');
  // 存 `key` 那一格（几项一行的照项各存各的）；和默认一样的交 null。
  const saveKey = async (key, value) => {
    failed.textContent = '';
    const fallback = inherited(one, key).value;
    const same = JSON.stringify(value) === JSON.stringify(fallback);
    const sent = values(key, same ? null : value);
    try {
      const answer = await context.request('cards.save_one', { card: cardOf(one), values: sent });
      if (answer.saved) return;
      failed.textContent = answer.problems?.[0]?.message || answer.message || say('failed/save');
    } catch {
      toast(say('failed/save'));
    }
  };
  const save = (value) => saveKey(field.key, value);
  name.append(mark(field, one, saveKey));
  line.append(label, control(field, one, context, save, saveKey));
  const page = EDIT_PAGES[field.control];
  if (page) {
    const edit = element('button', 'icon-button');
    edit.title = say(`venue/edit-${page}`);
    edit.append(icon('i-up-right'));
    edit.addEventListener('click', () => goTo(page));
    line.append(edit);
  }
  const holder = element('div', 'field');
  holder.append(line, failed);
  return holder;
}
