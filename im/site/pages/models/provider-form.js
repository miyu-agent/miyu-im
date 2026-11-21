// 编辑、手写添加一家供应商的表（设计 09，施工 I-149；照网页软件 provider-form.js）：ID（新建时写）、显示名、地址、接口、密钥（粘贴
// 或者环境变量）。存的先后：粘贴了密钥的先 secret.set 存成新名字，再一条 config.set 写别的几项和密钥的引用；config.set 拒了的，
// 已经存下的密钥名字写在报错后面。明文只在框里，存完、取消就扔。
//
// 只画这一家真有的项（施工 I-271，照网页软件 provider-fields.js，核心 FX-11）：`model.list` 每家的 `needs` 写 `none` 的那一项不出那一行
// （Antigravity 这种扩展驱动没有地址、不要密钥）；旧核心没有 `needs` 的、新建的照旧都画。软件包带的一家，包给的地址框空着、暗的占位字
// 写包给的地址，空着存不写；包给的接口人没换的不写（原来照选着的写进配置，存一次就把包给的接口抄进配置、盖住了包的）。
import { element, say } from '../../shell/i18n.js';
import { openDialog } from '../../shell/dialog.js';
import { core, refusalText } from '../../shell/core.js';
import { envRef, inputText, packageValue, providerChanges, providerName, secretName, shows, validId } from './config.js';
import { menuSelect } from '../../shell/menu.js';

/** 开表：`id` 是要编辑的那一家，null 是新建；存好了调 `done(id)`。 */
export function openProviderForm(state, id, done) {
  const providers = state.models?.providers ?? [];
  const editing = id;
  const provider = providers.find((one) => one.id === editing) ?? null;
  const title = editing ? say('models/form-edit-title', { name: providerName(provider ?? { id: editing }) }) : say('models/form-new-title');
  const dialog = openDialog(title);
  dialog.body.classList.add('persona-body');
  const input = (attrs) => {
    const box = element('input', 'input persona-input');
    Object.assign(box, { type: 'text', spellcheck: false, autocomplete: 'off', ...attrs });
    return box;
  };
  const idField = editing ? element('strong', null, editing) : input({ placeholder: say('models/form-id-hint') });
  const written = provider?.name && typeof provider.name === 'object' && provider.name.from === 'config' ? provider.name.value : '';
  const fallback = provider && typeof provider.name === 'object' && provider.name.from !== 'config' ? provider.name.value : editing;
  const nameField = input({
    value: written,
    placeholder: fallback ? say('models/form-name-empty', { name: fallback }) : say('models/form-name-empty-new'),
  });
  const needs = provider?.needs ?? {};
  // 地址是环境变量引用的、软件包给的：框空着就是不改。
  const urlEnv = envRef(provider?.base_url);
  const urlPackaged = editing ? packageValue(state.got, editing, 'base_url') : null;
  const urlKept = Boolean(urlEnv) || urlPackaged !== null;
  const urlField = shows(needs.base_url)
    ? input({
      value: urlKept ? '' : inputText(provider?.base_url ?? ''),
      placeholder: urlEnv ? say('models/from-env', { name: urlEnv }) : urlPackaged !== null ? inputText(urlPackaged) : 'https://…',
    })
    : null;
  const packagedDriver = editing ? packageValue(state.got, editing, 'driver') : null;
  const drivers = state.schema?.items.find((one) => one.key === 'providers.<id>.driver')?.options ?? [];
  const driverField = menuSelect(
    drivers.map((option) => ({ value: option.value, text: option.name ?? option.value })),
    provider?.driver ?? drivers[0]?.value ?? '',
  );
  // 密钥：粘贴（密码框，编辑时空着是不改）或者环境变量的名字。
  const keyRef = provider?.key?.ref ?? '';
  let keyKind = keyRef.startsWith('env:') ? 'env' : 'secret';
  const secretField = input({ type: 'password', placeholder: provider?.key?.set ? say('models/form-key-keep') : say('models/form-key-paste') });
  const envField = input({ value: keyRef.startsWith('env:') ? keyRef.slice(4) : '', placeholder: 'DEEPSEEK_API_KEY' });
  const keyBox = element('div');
  const kinds = element('div', 'seg');
  const kindButtons = ['secret', 'env'].map((kind) => {
    const button = element('button', null, say(`models/form-key-${kind}`));
    button.addEventListener('click', () => {
      keyKind = kind;
      drawKey();
    });
    return button;
  });
  kinds.append(...kindButtons);
  const drawKey = () => {
    kindButtons.forEach((button, at) => button.classList.toggle('on', ['secret', 'env'][at] === keyKind));
    keyBox.replaceChildren(keyKind === 'secret' ? secretField : envField);
  };
  drawKey();

  const error = element('p', 'persona-note');
  error.hidden = true;
  const save = element('button', 'btn primary', say('dialog/save'));
  const fail = (why) => {
    error.textContent = why;
    error.hidden = false;
    save.disabled = false;
  };
  save.addEventListener('click', async () => {
    error.hidden = true;
    const chosen = editing ?? idField.value.trim();
    if (!editing && !validId(chosen)) return fail(say('models/form-bad-id'));
    if (!editing && providers.some((one) => one.id === chosen)) return fail(say('models/form-taken-id', { id: chosen }));
    save.disabled = true;
    let secret = null;
    if (keyKind === 'secret' && secretField.value) {
      secret = secretName(chosen, Date.now());
      try {
        await core(state.request, 'secret.set', { name: secret, value: secretField.value });
      } catch (refused) {
        return fail(refusalText(refused));
      }
      secretField.value = '';
    }
    const form = {
      name: nameField.value,
      base_url: !urlField || (urlKept && !urlField.value.trim()) ? null : urlField.value,
      driver: typeof packagedDriver === 'string' && driverField.value === packagedDriver ? null : driverField.value,
      key: { kind: !keyRow || (keyKind === 'secret' && !secret) ? 'keep' : keyKind, value: envField.value },
    };
    const why = await state.saveMany(providerChanges(chosen, form, state.got, secret));
    if (why) return fail(secret ? `${why}\n${say('models/form-secret-kept', { name: secret })}` : why);
    dialog.close();
    done(chosen);
  });
  const cancel = element('button', 'btn', say('dialog/cancel'));
  cancel.addEventListener('click', () => dialog.close());
  const row = (label, control) => {
    const box = element('label', 'persona-field');
    box.append(element('span', 'persona-label', say(label)), control);
    return box;
  };
  const keyRow = shows(needs.key) ? element('div', 'persona-field') : null;
  keyRow?.append(element('span', 'persona-label', say('models/form-key')), kinds, keyBox);
  dialog.body.append(
    row('models/form-id', idField),
    row('models/form-name', nameField),
    ...(urlField ? [row('models/form-url', urlField)] : []),
    row('models/form-driver', driverField),
    ...(keyRow ? [keyRow] : []),
    error,
  );
  dialog.foot.append(element('span', 'grow'), cancel, save);
  (editing ? nameField : idField).focus();
}
