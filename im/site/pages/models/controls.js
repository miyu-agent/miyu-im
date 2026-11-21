// 配置项的一行（设计 09，施工 I-149；照网页软件 rows.js 的 coreRow，只做模型页用得上的几种控件）：左边名字，右边控件，这一层写过的
// 多一个「恢复默认」。控件照 config.schema 的 control：select 下拉、number 数、text 一行字、list（有选项的）一排勾选、toggle 开关；
// 改了一条 config.set 存（带 expect），存不成的写在这一行下面。没写的照 schema 的默认、模型资料的值显示。
import { element, icon, say } from '../../shell/i18n.js';
import { expectFor, inputText, writtenIn } from './config.js';
import { menuSelect, openMenu } from '../../shell/menu.js';

/** 一行：`item` 是 itemFor 的（schema 的那一项加真键、读到的 entry、模型资料 fact），`label` 盖掉 schema 的名字。存经 `state.saveMany`。 */
export function configRow(state, item, label) {
  const line = element('div', 'model-row');
  const note = element('p', 'persona-note');
  note.hidden = true;
  const value = item.entry?.value ?? item.fact?.value ?? item.default ?? null;
  const save = async (change) => {
    note.hidden = true;
    const why = await state.saveMany([{ key: item.key, ...change, expect: expectFor(item.entry) }]);
    if (why) {
      note.textContent = why;
      note.hidden = false;
    }
  };
  const name = element('span', 'model-row-name', label ?? item.name ?? item.key);
  const right = element('span', 'model-row-control');
  right.append(control(item, value, save));
  if (writtenIn(item.entry)) {
    const reset = element('button', 'icon-button');
    reset.title = say('models/reset');
    reset.append(icon('i-x'));
    reset.addEventListener('click', () => save({ unset: true }));
    right.append(reset);
  }
  line.append(name, right);
  const box = element('div');
  box.append(line, note);
  return box;
}

/** 控件。 */
function control(item, value, save) {
  switch (item.control) {
    case 'select': {
      const shown = menuSelect(
        (item.options ?? []).map((option) => ({ value: JSON.stringify(option.value), text: option.name ?? String(option.value) })),
        JSON.stringify(value),
      );
      shown.addEventListener('change', () => {
        const picked = JSON.parse(shown.value);
        save(picked === null ? { unset: true } : { value: picked });
      });
      return shown;
    }
    case 'toggle': {
      const button = element('button', value ? 'switch on' : 'switch');
      button.append(element('i'));
      button.addEventListener('click', () => save({ value: !button.classList.contains('on') }));
      return button;
    }
    case 'list':
      if (item.options?.length) return checks(item.options, Array.isArray(value) ? value : [], save);
      return text(Array.isArray(value) ? value.join(', ') : inputText(value), (typed) => save({ input: typed }));
    default:
      return text(inputText(value), (typed) => save(typed ? { input: typed } : { unset: true }), item.control === 'number');
  }
}

/** 一排选了的（施工 I-272，照网页包的「支持输入」）：只列选了的，各带一个 ×；最后一个 + 弹出没选的那几项，点一项加上。存整份列表。 */
function checks(options, chosen, save) {
  const box = element('span', 'model-checks');
  const now = new Set(chosen);
  const store = () => save({ value: options.map((one) => one.value).filter((one) => now.has(one)) });
  for (const option of options.filter((one) => now.has(one.value))) {
    const chip = element('span', 'chip model-check');
    const remove = element('button', 'model-check-x');
    remove.type = 'button';
    remove.title = say('models/input-remove');
    remove.append(icon('i-x'));
    remove.addEventListener('click', () => {
      now.delete(option.value);
      store();
    });
    chip.append(document.createTextNode(option.name ?? option.value), remove);
    box.append(chip);
  }
  const rest = options.filter((one) => !now.has(one.value));
  if (rest.length) {
    const add = element('button', 'chip model-check model-check-add');
    add.type = 'button';
    add.title = say('models/input-add');
    add.append(icon('i-plus'));
    add.addEventListener('click', () =>
      openMenu(add, rest.map((one) => ({ value: one.value, text: one.name ?? one.value })), null, (picked) => {
        now.add(rest.find((one) => String(one.value) === picked)?.value ?? picked);
        store();
      }),
    );
    box.append(add);
  }
  return box;
}

/** 一格字：回车、离开框存（没改的不存）。 */
export function text(value, commit, numeric = false) {
  const box = element('input', numeric ? 'input num' : 'input');
  if (numeric) box.inputMode = 'decimal';
  box.value = value;
  let last = value;
  const done = () => {
    const typed = box.value.trim();
    if (typed === last) return;
    last = typed;
    commit(typed);
  };
  box.addEventListener('keydown', (event) => {
    if (event.key === 'Enter' && !event.isComposing) done();
  });
  box.addEventListener('blur', done);
  return box;
}
