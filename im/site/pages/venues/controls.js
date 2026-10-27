// 会话浮窗各页的控件（施工 I-7 中一起，从 form.js 挪出来；中二多关键词、管理的人、概率、小数、几项一行）：照 fields.js 的 control
// 画，改了调 `save(value)`（null 是恢复默认），几项一行的照项各存各的（`saveKey(key, value)`）。没写的显示继承的值：下拉多一项、
// 输入框写在占位字里、开关照现在用着的画。选模型、贴表情点开小窗选（pickers.js），限流、睡眠在 spans.js，标签是 shell/tags.js。
import { element, say } from '../../shell/i18n.js';
import { tagInput } from '../../shell/tags.js';
import { current, inherited, written } from './fields.js';
import { modelControl, reactionControl } from './pickers.js';
import { rate, seconds, sleep } from './spans.js';

/** 控件。`save(value)` 存（null 是恢复默认）。 */
export function control(field, one, context, save, saveKey) {
  const mine = written(one, field.key);
  const fallback = inherited(one, field.key).value;
  switch (field.control) {
    case 'switch': {
      // 开、关存成什么：不写的是真、假（线路规程是 `chatty`、`when-called`）。
      const [yes, no] = field.values || [true, false];
      const button = element('button', (current(one, field.key) ?? field.unset) === yes ? 'switch on' : 'switch');
      button.append(element('i'));
      button.addEventListener('click', () => {
        const next = !button.classList.contains('on');
        button.classList.toggle('on', next);
        save(next ? yes : no);
      });
      return button;
    }
    case 'tags':
      return tags(field, one, context, save);
    case 'percent':
      return percent(field, one, save);
    case 'reaction':
      return reactionControl(field, one, context.reactions, save);
    case 'float':
      return float(mine, fallback, save);
    case 'multi': {
      const box = element('span', 'inline');
      for (const key of field.keys) box.append(float(written(one, key), inherited(one, key).value, (value) => saveKey(key, value), 'narrow'));
      return box;
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
    case 'preset': {
      // 预设（宿主 `presets` 交 {presets: [{id, name}]}；问不到的这一格不画，form.js 照 `needs`）。
      const names = new Map((context.presets?.presets || []).map((one) => [one.id, one.name || one.id]));
      if (mine && !names.has(mine)) names.set(mine, mine);
      return select([...names], mine, say('field/inherit', { value: fallback == null ? '—' : names.get(fallback) || fallback }), save);
    }
    case 'model':
      return modelControl(field, mine, fallback, context.models, save, field.orKey && current(one, field.orKey));
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

/** 字的列表（关键词、管理的人）：shell/tags.js 画，没写的照继承的画淡的标签、在它上面接着加。管理的人照号写（宿主把只有号的
 *  补成平台上的身份），认得的写名字；`prefixed` 的只填了号的补成平台上的人的编号。 */
function tags(field, one, context, save) {
  const people = new Map((context.friends || []).map((friend) => [String(friend.id), friend.name]));
  const shown = (item) => {
    const number = String(item).split(':').pop();
    const name = field.people ? people.get(number) : null;
    return name ? `${name} · ${number}` : field.people ? number : String(item);
  };
  return tagInput({
    items: written(one, field.key) ?? null,
    inherited: inherited(one, field.key).value || [],
    shown,
    normalize: (typed) => (field.prefixed && /^\d+$/.test(typed) ? `${context.platform}:${typed}` : typed),
    placeholder: say(field.people ? 'field/add-person' : 'field/add-word'),
    save,
  });
}

/** 概率：拖动条，百分之几，存成千分比（5% 存 50）。 */
function percent(field, one, save) {
  const box = element('span', 'inline');
  const slider = element('input', 'slider');
  slider.type = 'range';
  slider.min = '0';
  slider.max = String(field.max ?? 100);
  slider.step = '0.5';
  const value = current(one, field.key) ?? 0;
  slider.value = String(value / 10);
  const shown = element('b', 'slider-value', `${value / 10}%`);
  slider.addEventListener('input', () => (shown.textContent = `${slider.value}%`));
  slider.addEventListener('change', () => save(Math.round(Number(slider.value) * 10)));
  box.append(slider, shown);
  return box;
}

/** 小数（门槛、权重）：空的是恢复默认，读不出数的照字交给宿主报错。 */
function float(mine, fallback, save, narrow) {
  const input = element('input', narrow ? 'input num narrow' : 'input num');
  input.inputMode = 'decimal';
  input.value = mine === undefined ? '' : String(mine);
  input.placeholder = fallback == null ? '' : String(fallback);
  input.addEventListener('change', () => {
    const text = input.value.trim();
    if (text === '') return save(null);
    const number = Number(text);
    return save(Number.isFinite(number) ? number : text);
  });
  return input;
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
