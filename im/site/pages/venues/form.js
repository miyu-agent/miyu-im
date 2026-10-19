// 会话浮窗一页的格（施工 I-7 中一）：照 fields.js 画一行一格，改了当场存（cards.save_one，和默认一样的交 null，卡上只写和默认不一样
// 的），存好了宿主推 cards、浮窗拿新的那一张重画。存不成的在那一行下面写一句。
//
// - 没写的格显示「继承（下面几层的值）」：下拉多一项、输入框写在占位字里、开关照现在用着的画。
// - 开关管着的几行（dep），开关关着时藏起来；「高级」的收进一个默认收起的折叠里。
import { element, say, spaced } from '../../shell/i18n.js';
import { toast } from '../../shell/dialog.js';
import { FIELDS, current, has, inherited, values } from './fields.js';
import { cardOf } from './model.js';
import { control } from './controls.js';

/** 画 `tab` 那一页：`one` 是这张卡，`context` 是 { request, personas, models, expanded }（expanded 记着哪几页的「高级」展开着，
 *  重画时照旧）。交回几块元素。 */
export function drawForm(tab, one, context) {
  const rows = [];
  const advanced = [];
  for (const field of FIELDS[tab] || []) {
    if (!has(field, one)) continue;
    if (field.group) {
      rows.push(element('div', 'group-name', say(field.group)));
      continue;
    }
    if (field.dep && (current(one, field.dep.key) ?? field.dep.unset ?? field.dep.on) !== field.dep.on) continue;
    (field.advanced ? advanced : rows).push(row(field, one, context));
  }
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
    rows.push(details);
  }
  return rows;
}

/** 一行：名字、小字、控件，下面一行写存不成的原因。 */
function row(field, one, context) {
  const line = element('div', field.wide ? 'row wide' : 'row');
  const label = element('div', 'label', spaced(say(field.label, { id: context.idWord })));
  if (field.hint) label.append(element('small', null, say(field.hint)));
  const failed = element('div', 'row-failed');
  // 存 `key` 那一格（几项一行的照项各存各的）；和默认一样的交 null。这一格是 `clears` 的（线路规程）一起去掉卡上旧写法的那几格。
  const saveKey = async (key, value) => {
    failed.textContent = '';
    const fallback = inherited(one, key).value;
    const same = JSON.stringify(value) === JSON.stringify(fallback);
    const sent = values(key, same ? null : value);
    for (const old of key === field.key ? field.clears || [] : []) sent[old] = null;
    try {
      const answer = await context.request('cards.save_one', { card: cardOf(one), values: sent });
      if (answer.saved) return;
      failed.textContent = answer.problems?.[0]?.message || answer.message || say('failed/save');
    } catch {
      toast(say('failed/save'));
    }
  };
  const save = (value) => saveKey(field.key, value);
  line.append(label, control(field, one, context, save, saveKey));
  const holder = element('div', 'field');
  holder.append(line, failed);
  return holder;
}
