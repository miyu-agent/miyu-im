// 会话浮窗各页的控件（施工 I-7 中一起，从 form.js 挪出来；中二多关键词、管理的人、概率、小数、几项一行）：照 fields.js 的 control
// 画，改了调 `save(value)`（null 是恢复默认），几项一行的照项各存各的（`saveKey(key, value)`）。没写的显示继承的值：下拉多一项、
// 输入框写在占位字里、开关照现在用着的画。
import { element, icon, say } from '../../shell/i18n.js';
import { current, inherited, written } from './fields.js';

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
      return reaction(field, one, context.reactions, save);
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

/** 字的列表（关键词、管理的人）：一个一个标签，× 去掉一个，最后一格填了按回车加一个。没写的照继承的画淡的标签。管理的人照号写
 *  （宿主把只有号的补成平台上的身份），认得的写名字。 */
function tags(field, one, context, save) {
  const box = element('div', 'tags');
  const mine = written(one, field.key);
  const list = mine ?? inherited(one, field.key).value ?? [];
  const people = new Map((context.friends || []).map((friend) => [String(friend.id), friend.name]));
  const shown = (item) => {
    const number = String(item).split(':').pop();
    const name = field.people ? people.get(number) : null;
    return name ? `${name} · ${number}` : field.people ? number : String(item);
  };
  list.forEach((item, at) => {
    const tag = element('span', mine === undefined ? 'tag inherited' : 'tag', shown(item));
    if (mine !== undefined) {
      const remove = element('button');
      remove.title = say('field/remove');
      remove.append(icon('i-x'));
      remove.addEventListener('click', () => save(list.filter((_, index) => index !== at)));
      tag.append(remove);
    }
    box.append(tag);
  });
  const adding = element('input', 'input tag-add');
  adding.placeholder = say(field.people ? 'field/add-person' : 'field/add-word');
  // 加了的先画上、清空这一格接着加（还在这一格里时浮窗不重画，离开了照宿主的重画）。
  const added = [...(mine ?? [])];
  adding.addEventListener('keydown', (event) => {
    if (event.key !== 'Enter' || adding.value.trim() === '') return;
    const typed = adding.value.trim();
    const item = field.prefixed && /^\d+$/.test(typed) ? `${context.platform}:${typed}` : typed;
    added.push(item);
    adding.before(element('span', 'tag', shown(item)));
    adding.value = '';
    save([...added]);
  });
  box.append(adding);
  return box;
}

/** 贴表情（施工 I-7 中三下二）：开关加选哪个。规则写表情的编号，`off` 是不贴；没写的照桥的出厂（`known.default`）。关掉存
 *  `off`，打开存选着的那一个。 */
function reaction(field, one, known, save) {
  const box = element('span', 'inline');
  const now = current(one, field.key);
  const on = now !== 'off';
  const chosen = on && now ? now : known.default;
  const button = element('button', on ? 'switch on' : 'switch');
  button.append(element('i'));
  const pick = element('select', 'select');
  for (const one of known.reactions) pick.append(new Option(one.glyph ? `${one.glyph} ${one.name}` : one.name, one.id));
  if (!known.reactions.some((one) => one.id === chosen)) pick.append(new Option(chosen, chosen));
  pick.value = chosen;
  pick.hidden = !on;
  button.addEventListener('click', () => {
    const next = !button.classList.contains('on');
    button.classList.toggle('on', next);
    pick.hidden = !next;
    save(next ? pick.value : 'off');
  });
  pick.addEventListener('change', () => save(pick.value));
  box.append(pick, button);
  return box;
}

/** 概率：拖动条，百分之几，存成千分比（5% 存 50）。 */
function percent(field, one, save) {
  const box = element('span', 'inline');
  const slider = element('input', 'slider');
  slider.type = 'range';
  slider.min = '0';
  slider.max = String(field.max ?? 30);
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
