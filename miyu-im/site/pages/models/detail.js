// 一个模型的设置（设计 09，施工 I-149；照网页软件 model-detail.js）：点供应商那一栏的一个模型开浮窗（网页软件是从右边滑进来的抽屉，
// im 网页用浮窗）：支持输入、上下文窗口、默认思考强度（只列这个模型报的那几档，前面一个「默认」是删掉）、默认温度（这个模型收温度的
// 才有）、价格（四格加币种，改一项照显示的整份存，空着的是删掉，不当成 0）。存完页面重读，浮窗照新读到的重画。
import { element, say } from '../../shell/i18n.js';
import { openDialog } from '../../shell/dialog.js';
import { expectFor, inputText, itemFor, modelName, writtenIn } from './config.js';
import { configRow, text } from './controls.js';

/** 开供应商 `providerId` 的模型 `modelId` 的设置：`state` 是 modelState，重读以后调 `state.onDetail`。 */
export function openModel(state, providerId, modelId) {
  const dialog = openDialog('', () => {
    state.onDetail = null;
  });
  dialog.body.classList.add('persona-body');
  const draw = () => {
    const provider = state.models?.providers?.find((one) => one.id === providerId);
    const model = provider?.models?.find((one) => one.model === modelId);
    if (!model || !state.schema || !state.got) {
      dialog.close();
      return;
    }
    dialog.setTitle(modelName(model));
    const facts = model.facts ?? {};
    const names = { id: providerId, model: modelId };
    const item = (field, fact) =>
      itemFor(state.schema, state.got, `providers.<id>.models.<model>.${field}`, names, fact ? { value: fact.value } : undefined);
    const rows = [element('code', 'model-id', modelId)];
    const add = (field, fact, label) => {
      const shown = item(field, fact);
      if (shown) rows.push(configRow(state, shown, say(label)));
    };
    add('inputs', facts.inputs, 'models/inputs');
    add('window', facts.window, 'models/window');
    const effort = item('effort');
    const levels = facts.reasoning?.value ?? [];
    if (effort && levels.length) {
      const options = [{ value: null, name: say('models/default') }, ...levels.map((one) => ({ value: one, name: one }))];
      rows.push(configRow(state, { ...effort, control: 'select', options }, say('models/effort')));
    }
    if (facts.takes_temperature?.value !== false) add('temperature', undefined, 'models/temperature');
    rows.push(prices(state, item));
    dialog.body.replaceChildren(...rows);
  };
  state.onDetail = draw;
  draw();
}

/** 价格一块：输入、输出、缓存读、缓存写、币种；改一项五项照显示的整份写，空着的这一层写过才删。 */
function prices(state, item) {
  const parts = ['input', 'output', 'cache_read', 'cache_write', 'currency']
    .map((name) => ({ name, shown: item(`price.${name}`) }))
    .filter((one) => one.shown);
  const inputs = new Map();
  const note = element('p', 'persona-note');
  note.hidden = true;
  const saveAll = async () => {
    const changes = parts.flatMap(({ name, shown }) => {
      const typed = inputs.get(name)?.value.trim() ?? '';
      const expect = expectFor(shown.entry);
      if (typed) return [{ key: shown.key, input: typed, expect }];
      return writtenIn(shown.entry) ? [{ key: shown.key, unset: true, expect }] : [];
    });
    const why = await state.saveMany(changes);
    note.hidden = !why;
    note.textContent = why ?? '';
  };
  const grid = element('div', 'model-prices');
  for (const { name, shown } of parts) {
    const cell = element('label', 'model-price');
    const box = text(inputText(shown.entry?.value ?? ''), saveAll, name !== 'currency');
    inputs.set(name, box);
    cell.append(element('span', 'persona-label', say(`models/price-${name.replace('_', '-')}`)), box);
    grid.append(cell);
  }
  const box = element('section', 'persona-field');
  box.append(element('span', 'persona-label', say('models/price')), grid, note);
  return box;
}
