// 会话浮窗一页的格（施工 I-7 中一）：照 fields.js 画一行一格，改了当场存（cards.save_one，和默认一样的交 null，卡上只写和默认不一样
// 的），存好了宿主推 cards、浮窗拿新的那一张重画。存不成的在那一行下面写一句。
//
// - 没写的格显示「继承（下面几层的值）」：下拉多一项、输入框写在占位字里、开关照现在用着的画。
// - 开关管着的几行（dep），开关关着时藏起来；「高级」的收进一个默认收起的折叠里。
import { element, say } from '../../shell/i18n.js';
import { toast } from '../../shell/dialog.js';
import { FIELDS, current, has, inherited, values, written } from './fields.js';
import { cardOf } from './model.js';

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
    if (field.dep && (depValue(one, field.dep.key) ?? field.dep.on) !== field.dep.on) continue;
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

/** 管着别的格的那一格现在的值（线路规程的旧写法照卡上的插嘴算）。 */
function depValue(one, key) {
  if (key === 'rules.discipline' && written(one, key) === undefined && one.set.chatty) {
    return one.set.chatty === 'never' ? 'when-called' : 'chatty';
  }
  return current(one, key);
}

/** 一行：名字、小字、控件，下面一行写存不成的原因。 */
function row(field, one, context) {
  const line = element('div', field.wide ? 'row wide' : 'row');
  const label = element('div', 'label', say(field.label));
  if (field.hint) label.append(element('small', null, say(field.hint)));
  const failed = element('div', 'row-failed');
  const save = async (value) => {
    failed.textContent = '';
    const fallback = inherited(one, field.key).value;
    const same = JSON.stringify(value) === JSON.stringify(fallback);
    try {
      const answer = await context.request('cards.save_one', { card: cardOf(one), values: values(field.key, same ? null : value) });
      if (answer.saved) return;
      failed.textContent = answer.problems?.[0]?.message || answer.message || say('failed/save');
    } catch {
      toast(say('failed/save'));
    }
  };
  line.append(label, control(field, one, context, save));
  const holder = element('div', 'field');
  holder.append(line, failed);
  return holder;
}

/** 控件。`save(value)` 存（null 是恢复默认）。 */
function control(field, one, context, save) {
  const mine = written(one, field.key);
  const fallback = inherited(one, field.key).value;
  switch (field.control) {
    case 'switch': {
      const button = element('button', current(one, field.key) ? 'switch on' : 'switch');
      button.append(element('i'));
      button.addEventListener('click', () => {
        const next = !button.classList.contains('on');
        button.classList.toggle('on', next);
        save(next);
      });
      return button;
    }
    case 'select':
      return select(
        field.options.map((value) => [value, say(`${field.optionText}${value}`)]),
        mine,
        say('field/inherit', { value: fallback == null ? '—' : say(`${field.optionText}${fallback}`) }),
        save,
      );
    case 'persona': {
      const names = new Map(context.personas.map((one) => [one.persona, one.name || one.persona]));
      const shown = fallback == null ? say('field/inherit-persona') : say('field/inherit', { value: names.get(fallback) || fallback });
      return select([...names], mine, shown, save);
    }
    case 'model': {
      const models = context.models || { models: [], pools: [], uses: {} };
      const options = [
        ...models.models.map((model) => [model.ref, model.state ? `${model.ref} · ${say('field/model-unusable')}` : model.ref]),
        ...models.pools.map((pool) => [`@${pool}`, `@${pool}`]),
      ];
      if (mine && !options.some(([value]) => value === mine)) options.push([mine, mine]);
      const used = fallback ?? models.uses?.[field.use] ?? null;
      return select(options, mine, say('field/inherit', { value: used ?? '—' }), save);
    }
    case 'area': {
      const area = element('textarea', 'area');
      area.value = mine ?? '';
      area.placeholder = fallback ?? '';
      area.addEventListener('change', () => save(area.value.trim() === '' ? null : area.value));
      return area;
    }
    case 'number':
    case 'seconds': {
      const box = element('span', 'inline');
      const input = element('input', 'input num');
      input.inputMode = 'numeric';
      const toNumber = (value) => (field.control === 'seconds' ? seconds(value) : value);
      input.value = mine === undefined ? '' : String(toNumber(mine));
      input.placeholder = fallback == null ? '' : String(toNumber(fallback));
      input.addEventListener('change', () => {
        const text = input.value.trim();
        if (text === '') return save(null);
        const number = Number(text);
        if (!Number.isInteger(number)) return save(text);
        return save(field.control === 'seconds' ? `${number}s` : number);
      });
      box.append(input, document.createTextNode(say(field.unit || 'unit/seconds')));
      return box;
    }
    case 'rate':
      return rate(mine, fallback, save);
    case 'sleep':
      return sleep(mine, fallback, save);
    default:
      return element('span');
  }
}

/** 下拉：第一项是「继承（…）」（交 null），别的照 `options`（[值, 给人看的]）。 */
function select(options, mine, inherit, save) {
  const shown = element('select', 'select');
  shown.append(new Option(inherit, ''));
  for (const [value, text] of options) shown.append(new Option(text, value));
  shown.value = mine ?? '';
  shown.addEventListener('change', () => save(shown.value === '' ? null : shown.value));
  return shown;
}

/** 限流：几条 / 几秒（规则写 `N/Ms`，`0` 是不限）。两格都空是恢复默认。 */
function rate(mine, fallback, save) {
  const box = element('span', 'inline');
  const turns = element('input', 'input num');
  const span = element('input', 'input num');
  const [fallTurns, fallSpan] = parseRate(fallback);
  const [myTurns, mySpan] = parseRate(mine);
  turns.value = mine === undefined ? '' : myTurns;
  span.value = mine === undefined ? '' : mySpan;
  turns.placeholder = fallTurns;
  span.placeholder = fallSpan;
  const changed = () => {
    const [count, length] = [turns.value.trim(), span.value.trim()];
    if (count === '' && length === '') return save(null);
    if (count === '0') return save('0');
    return save(`${count || fallTurns}/${length || fallSpan}s`);
  };
  turns.addEventListener('change', changed);
  span.addEventListener('change', changed);
  box.append(turns, document.createTextNode(say('unit/turns-per')), span, document.createTextNode(say('unit/seconds')));
  return box;
}

/** 睡眠：几点到几点（规则写 `HH:MM-HH:MM`，`off` 是不睡）。两格都空是恢复默认。 */
function sleep(mine, fallback, save) {
  const box = element('span', 'inline');
  const from = element('input', 'input time');
  const to = element('input', 'input time');
  from.type = 'time';
  to.type = 'time';
  const [myFrom, myTo] = typeof mine === 'string' && mine.includes('-') ? mine.split('-') : ['', ''];
  const [fallFrom, fallTo] = typeof fallback === 'string' && fallback.includes('-') ? fallback.split('-') : ['', ''];
  from.value = myFrom || '';
  to.value = myTo || '';
  from.title = fallFrom;
  to.title = fallTo;
  const changed = () => {
    if (!from.value && !to.value) return save(null);
    return save(`${from.value || fallFrom}-${to.value || fallTo}`);
  };
  from.addEventListener('change', changed);
  to.addEventListener('change', changed);
  box.append(from, document.createTextNode('–'), to);
  if (!mine && fallback) box.append(element('small', 'faint', say('field/inherit', { value: fallback === 'off' ? say('field/sleep-off') : fallback })));
  return box;
}

/** `5/300s` 拆成 ['5', '300']（时长换成秒）；不是这个写法的是两个空的，`0` 是 ['0', '']。 */
function parseRate(text) {
  if (typeof text !== 'string') return ['', ''];
  if (text === '0') return ['0', ''];
  const [count, length] = text.split('/');
  return [count || '', length ? String(seconds(length)) : ''];
}

/** 时长的写法（`30s`、`5m`、`1h`、`30`）换成秒；读不出的照原样。 */
function seconds(text) {
  const found = /^(\d+)([smh]?)$/.exec(String(text));
  if (!found) return text;
  const unit = { '': 1, s: 1, m: 60, h: 3600 }[found[2]];
  return Number(found[1]) * unit;
}
