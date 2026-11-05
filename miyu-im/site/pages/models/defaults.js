// 「模型」页的「默认模型」（设计 09，施工 I-152；照网页软件 defaults.js，2026-10-11 项目主人定：一个用途一行）：对话（models.chat）、
// 看图（models.vision）右边一个像下拉的按钮，点开按供应商分组的选择器（shell/picker.js，同会话卡片选模型；看图只列看得了图的）；
// 语义检索（models.embedding）右边一个下拉（内置模型、关，没装的暗着、点不了，选着的正是它时下面一行黄字）。选了当场存。
import { element, say } from '../../shell/i18n.js';
import { openPicker, pickButton } from '../../shell/picker.js';
import { expectFor, itemFor, modelName, providerName, sameNamed, seesImages, whereOf } from './config.js';
import { metaOf } from './bits.js';

/** 默认模型那一栏。 */
export function drawDefaults(state) {
  const box = element('div', 'model-main model-defaults');
  for (const row of [useRow(state, 'chat'), useRow(state, 'vision'), embeddingRow(state)]) if (row) box.append(row);
  return box;
}

/** 一行：左边名字、说明，右边控件，下面写不成的原因。交回 { el, fail(text) }。 */
function line(name, description, control) {
  const row = element('div', 'model-row');
  const text = element('div', 'model-row-name');
  text.append(element('span', null, name));
  if (description) text.append(element('small', 'faint', description));
  row.append(text, control);
  const note = element('p', 'persona-note');
  note.hidden = true;
  const el = element('div');
  el.append(row, note);
  return {
    el,
    fail(why) {
      note.hidden = !why;
      note.textContent = why ?? '';
    },
  };
}

/** 对话、看图。 */
function useRow(state, which) {
  const item = itemFor(state.schema, state.got, `models.${which}`, {});
  if (!item) return null;
  const current = item.entry?.value ?? null;
  const providers = state.models?.providers ?? [];
  const found = providers.flatMap((one) => (one.models ?? []).map((model) => ({ provider: one, model }))).find(({ model }) => model.ref === current);
  const same = sameNamed(providers);
  const shown = found
    ? `${modelName(found.model)} · ${same.has(found.provider.id) ? `${providerName(found.provider)} ${whereOf(found.provider.base_url) ?? ''}`.trim() : providerName(found.provider)}`
    : current ? String(current) : say('models/unset');
  let row;
  const button = pickButton(shown, () =>
    openPicker({
      title: say(`models/use-${which}`),
      placeholder: say('models/search'),
      groups: providers
        .map((provider) => ({
          name: providerName(provider),
          items: (provider.models ?? [])
            .filter((model) => which !== 'vision' || seesImages(model))
            .filter((model) => !model.state || model.state === 'ok' || model.state === 'cooling')
            .map((model) => ({ value: model.ref, text: modelName(model), sub: metaOf(model) })),
        }))
        .filter((group) => group.items.length),
      current,
      pick: async (ref) => {
        if (ref === current) return;
        row.fail(await state.saveMany([{ key: item.key, value: ref, expect: expectFor(item.entry) }]));
      },
    }),
  );
  row = line(say(`models/use-${which}`), say(`models/use-${which}-desc`), button);
  return row.el;
}

/** 语义检索：没写的照内置模型（核心：不写就是 local）；写的是一家供应商的模型（以前在别处写的）照写着列出来。 */
function embeddingRow(state) {
  const item = itemFor(state.schema, state.got, 'models.embedding', {});
  if (!item) return null;
  const written = item.entry?.value ?? null;
  const current = typeof written === 'string' ? written : 'local';
  const options = (item.options ?? []).map((one) => ({ value: one.value, name: one.name ?? one.value, off: one.available === false }));
  if (!options.some((one) => one.value === current)) options.push({ value: current, name: current, off: false });
  const select = element('select', 'select');
  for (const one of options) {
    const option = new Option(one.off ? `${one.name}（${say('models/not-installed')}）` : one.name, one.value);
    option.disabled = one.off && one.value !== current;
    select.append(option);
  }
  select.value = current;
  let row;
  select.addEventListener('change', async () => {
    row.fail(await state.saveMany([{ key: item.key, value: select.value, expect: expectFor(item.entry) }]));
  });
  row = line(say('models/use-embedding'), item.description ?? '', select);
  const using = (item.options ?? []).find((one) => one.value === current);
  if (using?.available === false) {
    row.el.append(element('p', 'warn', say('models/embedding-missing', { name: using.name ?? using.value })));
  }
  return row.el;
}
