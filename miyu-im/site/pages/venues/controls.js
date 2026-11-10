// 会话浮窗各页的控件（施工 I-7 中一起，从 form.js 挪出来；中二多关键词、管理的人、小数、几项一行）：照 fields.js 的 control
// 画，改了调 `save(value)`（null 是恢复默认），几项一行的照项各存各的（`saveKey(key, value)`）。没写的显示继承的值：下拉多一项、
// 输入框写在占位字里、开关照现在用着的画。选模型、贴表情点开小窗选（pickers.js），限流、睡眠在 spans.js，标签是 shell/tags.js。
import { element, say } from '../../shell/i18n.js';
import { tagInput } from '../../shell/tags.js';
import { tokensOf } from '../../shell/amount.js';
import { current, inherit, inherited, written } from './fields.js';
import { modelControl, reactionControl } from './pickers.js';
import { rate, seconds, sleep } from './spans.js';
import { isDefaults } from './model.js';

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
        field.options
          .filter((value) => can(field.optionCaps?.[value], one, context) || value === mine)
          .map((value) => [value, say(`${field.optionText}${value}`)]),
        mine,
        inherit(one, fallback == null ? null : say(`${field.optionText}${fallback}`)),
        save,
      );
    case 'persona': {
      const names = new Map(context.personas.map((one) => [one.persona, one.name || one.persona]));
      const shown = inherit(one, fallback == null ? null : names.get(fallback) || fallback);
      return select([...names], mine, shown, save);
    }
    case 'preset': {
      // 预设（宿主 `presets` 交 {presets: [{id, name}]}；问不到的这一格不画，form.js 照 `needs`）。
      const names = new Map((context.presets?.presets || []).map((one) => [one.id, one.name || one.id]));
      if (mine && !names.has(mine)) names.set(mine, mine);
      return select([...names], mine, inherit(one, fallback == null ? null : names.get(fallback) || fallback), save);
    }
    case 'model':
      return modelControl(field, mine, fallback, context.models, save, field.orKey && current(one, field.orKey), (value) => inherit(one, value));
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
      // 存的和显示的差一个倍数的（施工 I-197：概率存千分之几、显示百分之几）：`scale` 是存的除以显示的。
      const scale = field.scale || 1;
      const toNumber = (value) => (field.control === 'seconds' ? seconds(value) : value / scale);
      input.value = mine === undefined ? '' : String(toNumber(mine));
      // 没有继承的值时写这一格空着是什么意思（施工 I-168：使用限制的「不限」）。
      input.placeholder = fallback == null ? (field.empty ? say(field.empty) : '') : String(toNumber(fallback));
      input.addEventListener('change', () => {
        const text = input.value.trim();
        if (text === '') return save(null);
        // 填 token 数的格认 k、m 的写法（施工 I-202）：上下文窗口、使用限制的三格。
        const number = field.unit === 'unit/tokens' ? tokensOf(text) : Math.round(Number(text) * scale * 1e6) / 1e6;
        if (!Number.isInteger(number)) return save(text);
        return save(field.control === 'seconds' ? `${number}s` : number);
      });
      box.append(input, document.createTextNode(say(field.unit || 'unit/seconds')));
      return box;
    }
    case 'rate':
      return rate(mine, fallback, save);
    case 'sleep':
      return sleep(mine, fallback, save, (value) => inherit(one, value));
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
    // 要填人的照好友提示（设计 11 G4，施工 I-168），选了当场加上。
    suggestions: field.people ? (context.friends || []).map((friend) => ({ id: friend.id, name: friend.name })) : undefined,
  });
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

/** 这张卡的连接做不做得到 `cap`（施工 I-85 补）：没要求的、会话默认值那张、不知道连接能做什么的都算做得到。 */
function can(cap, one, context) {
  return !cap || isDefaults(one) || !context.caps || context.caps.includes(cap);
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
