// 「模型」页的供应商（设计 09，施工 I-149；照网页软件 providers.js，2026-10-11 项目主人定的样子）：左边一家一格（图标、名字，下面用不了
// 的原因或者地址的主机名，同名的几家靠地址分开），最底下「＋ 添加供应商」；右边头上图标、名字、地址和「测试连接」「编辑」「删除」，
// 密钥一块，模型一个一行（名字、「1M 上下文 · 看图」），点一行开这个模型的设置（detail.js）。
//
// - 删除（2026-10-19 加，10-20 改弹窗问）：只在系统配置里写过这一家时出现，默认模型用着它的问的话里多一句；确认了 providers.<id>.* 各
//   unset。
// - 密钥：设了的一行「已设置」「来自环境变量 X」和「更换」；没设的直接一个框（粘贴或者改用环境变量）。粘贴的先 secret.set 存成新名字，
//   再 config.set 写引用。配置里没写密钥的（本机的服务这类）同样能补，不写「用不了」（2026-10-13 W-139）。
// - 还没有模型的一家自动请核心拉一次（这一页里每家一次）。测试连接成了，核心顺手存了列出来的模型，重读这一家。
import { element, icon, say } from '../../shell/i18n.js';
import { confirmFirst } from '../../shell/dialog.js';
import { core, refusalText } from '../../shell/core.js';
import { duplicates, hostOf, inputText, keyChange, modelName, providerName, providerRemoval, sameNamed, secretName, sharedState, whereOf } from './config.js';
import { hasLogos, loadingRow, logoElement, metaOf, searchable, testLine } from './bits.js';
import { openProviderForm } from './provider-form.js';
import { openModel } from './detail.js';

/** 供应商那一栏。`view` 是页面记着的（选中哪一家、搜的字、测试结果、正在改密钥的），改了调 `redraw()`；加一家调 `add()`。 */
export function drawProviders(state, view, redraw, add) {
  const all = state.models?.providers ?? [];
  const chosen = all.find((one) => one.id === view.provider) ?? all[0];
  const logos = hasLogos(all);
  const same = sameNamed(all);
  const list = element('div', 'model-providers');
  for (const one of all) list.append(item(one, one.id === chosen?.id, logos, same.has(one.id), () => {
    Object.assign(view, { provider: one.id, keyEdit: null, tested: null });
    view.query.providers = '';
    redraw();
  }));
  const plus = element('button', 'model-provider add');
  plus.append(icon('i-plus'), element('span', null, say('models/add-provider')));
  plus.addEventListener('click', add);
  list.append(plus);
  const wrap = element('div', 'model-wrap');
  wrap.append(list);
  if (!chosen) return wrap;
  const main = element('div', 'model-main');
  main.append(head(state, view, chosen, logos, redraw), keyBlock(state, view, chosen, redraw), models(state, view, chosen));
  wrap.append(main);
  return wrap;
}

/** 左边一格：图标、名字，下面一行用不了的原因（warn），能用的写地址的主机名；同名的多写一行地址从哪来。 */
function item(one, on, logos, same, pick) {
  const shared = sharedState(one.models ?? []);
  const reason = shared ? element('small', 'warn', say(`models/short-${shared.replace('_', '-')}`)) : null;
  const where = same ? whereOf(one.base_url) : reason ? null : hostOf(one.base_url);
  const button = element('button', on ? 'model-provider on' : 'model-provider');
  const text = element('span', 'model-provider-text');
  text.append(element('span', null, providerName(one)));
  if (where) text.append(element('small', null, where));
  if (reason) text.append(reason);
  if (logos) button.append(logoElement(one.logo ?? null, providerName(one)));
  button.append(text);
  button.addEventListener('click', pick);
  return button;
}

/** 头上：图标、名字、地址，「测试连接」「编辑」「删除」；接口是猜的写一行；测试的结果写在下面。 */
function head(state, view, provider, logos, redraw) {
  const address = provider.base_url?.env ? say('models/from-env', { name: provider.base_url.env }) : inputText(provider.base_url);
  const shown = view.tested?.id === provider.id ? view.tested : null;
  const result = element('p', shown?.tone ? `model-test ${shown.tone}` : 'model-test', shown?.text ?? say('models/testing'));
  result.hidden = !shown;
  const test = element('button', 'btn', say('models/test'));
  test.disabled = Boolean(shown) && shown.text == null;
  test.addEventListener('click', async () => {
    view.tested = { id: provider.id };
    redraw();
    try {
      const got = await core(state.request, 'provider.test', { provider: provider.id });
      view.tested = { id: provider.id, tone: got.ok ? 'good' : 'bad', text: testLine(got) };
      if (got.ok) await state.refreshProvider(provider.id, false);
    } catch (error) {
      view.tested = { id: provider.id, tone: 'bad', text: refusalText(error) };
    }
    redraw();
  });
  const edit = element('button', 'btn', say('models/edit'));
  edit.addEventListener('click', () => openProviderForm(state, provider.id, () => redraw()));
  const buttons = element('div', 'model-buttons');
  buttons.append(test, edit, ...removeButton(state, view, provider, redraw));
  const title = element('div', 'model-title');
  const named = element('div', 'model-title-text');
  named.append(element('strong', null, providerName(provider)), element('span', 'faint', address));
  if (logos) title.append(logoElement(provider.logo ?? null, providerName(provider)));
  title.append(named, buttons);
  const box = element('div', 'model-head');
  box.append(title);
  if (provider.driver_from === 'default') box.append(element('p', 'faint', say('models/driver-guess', { driver: provider.driver ?? '' })));
  const shared = sharedState(provider.models ?? []);
  if (shared && shared !== 'no_key') box.append(element('p', 'warn', say(`models/state-${shared.replace('_', '-')}`)));
  box.append(result);
  return box;
}

/** 「删除」：系统配置里写过这一家才有。 */
function removeButton(state, view, provider, redraw) {
  const removal = providerRemoval(provider.id, state.got ?? {});
  if (!removal.length) return [];
  const chat = state.got?.items?.['models.chat']?.value;
  const inUse = typeof chat === 'string' && chat.startsWith(`${provider.id}/`);
  const button = element('button', 'btn ghost danger', say('models/remove'));
  button.addEventListener('click', async () => {
    const name = providerName(provider);
    const text = say(inUse ? 'models/remove-in-use' : 'models/remove-ask', { name });
    if (!(await confirmFirst(say('models/remove'), text, say('models/remove')))) return;
    const why = await state.saveMany(removal);
    if (why) {
      view.tested = { id: provider.id, tone: 'bad', text: why };
    } else {
      view.provider = null;
    }
    redraw();
  });
  return [button];
}

/** 密钥一块。 */
function keyBlock(state, view, provider, redraw) {
  const key = provider.key ?? { set: false, ref: null };
  const unwritten = !provider.key;
  const changing = view.keyEdit === provider.id;
  if (key.set && !changing) {
    const line = element('div', 'model-key');
    const now = key.ref?.startsWith('env:') ? say('models/from-env', { name: key.ref.slice(4) }) : say('models/key-saved');
    const change = element('button', 'btn', say('models/key-change'));
    change.addEventListener('click', () => {
      view.keyEdit = provider.id;
      redraw();
    });
    line.append(element('span', 'persona-label', say('models/key')), element('span', null, now), change);
    return line;
  }
  const kind = view.keyKind;
  const field = element('input', 'input persona-input');
  Object.assign(field, kind === 'secret'
    ? { type: 'password', placeholder: say('models/form-key-paste'), autocomplete: 'off' }
    : { type: 'text', placeholder: 'DEEPSEEK_API_KEY', spellcheck: false, autocomplete: 'off' });
  const error = element('p', 'persona-note');
  error.hidden = true;
  const save = element('button', 'btn primary', say('dialog/save'));
  save.addEventListener('click', async () => {
    const typed = field.value.trim();
    if (!typed) return field.focus();
    save.disabled = true;
    error.hidden = true;
    let ref;
    if (kind === 'secret') {
      const name = secretName(provider.id, Date.now());
      try {
        await core(state.request, 'secret.set', { name, value: typed });
      } catch (refused) {
        save.disabled = false;
        error.textContent = refusalText(refused);
        error.hidden = false;
        return;
      }
      field.value = '';
      ref = { secret: name };
    } else {
      ref = { env: typed };
    }
    const why = await state.saveMany(keyChange(provider.id, ref, state.got));
    if (why) {
      save.disabled = false;
      error.textContent = why;
      error.hidden = false;
      return;
    }
    view.keyEdit = null;
    redraw();
  });
  field.addEventListener('keydown', (event) => {
    if (event.key === 'Enter' && !event.isComposing) save.click();
  });
  const swap = element('button', 'btn ghost', say(kind === 'secret' ? 'models/key-use-env' : 'models/key-use-paste'));
  swap.addEventListener('click', () => {
    view.keyKind = kind === 'secret' ? 'env' : 'secret';
    redraw();
  });
  const row = element('div', 'model-key-row');
  row.append(field, save);
  if (changing) {
    const cancel = element('button', 'btn', say('dialog/cancel'));
    cancel.addEventListener('click', () => {
      view.keyEdit = null;
      redraw();
    });
    row.append(cancel);
  }
  row.append(swap);
  const missing = !key.set && !unwritten;
  const box = element('div', missing ? 'model-key-block missing' : 'model-key-block');
  if (missing) box.append(element('p', 'warn', say('models/state-no-key')));
  if (unwritten) box.append(element('span', 'persona-label', say('models/key')));
  box.append(row, error);
  return box;
}

/** 模型：一行「模型 N」和搜索框，下面一个一行；点一行开设置。 */
function models(state, view, provider) {
  const all = provider.models ?? [];
  if (!all.length && !state.refreshed.has(provider.id)) queueMicrotask(() => state.refreshProvider(provider.id, true));
  if (!all.length) return state.loadingProvider === provider.id ? loadingRow() : element('p', 'faint', say('models/no-models'));
  const shared = sharedState(all);
  const twice = duplicates(all.map(modelName));
  const rows = all.map((model) => {
    const node = element('button', 'model-line');
    const name = element('span', 'model-line-name', modelName(model));
    if (twice.has(modelName(model))) name.append(element('code', 'model-id', model.model));
    node.append(name);
    if (!shared && model.state && model.state !== 'ok') node.append(element('em', 'warn', say(`models/state-${model.state.replace('_', '-')}`)));
    node.append(element('span', 'faint', metaOf(model, true)));
    node.addEventListener('click', () => openModel(state, provider.id, model.model));
    return { name: modelName(model), model: model.model, node };
  });
  const find = searchable(view.query, 'providers', rows);
  const top = element('div', 'model-list-head');
  top.append(element('span', null, say('models/count', { count: all.length })), find.input);
  const list = element('div', 'model-lines');
  list.append(...rows.map((row) => row.node));
  const box = element('section', 'model-list');
  box.append(top, list, find.empty);
  return box;
}
