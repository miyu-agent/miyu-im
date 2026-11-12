// 编辑、手写添加一家供应商的表（设计 09，施工 I-149；照网页软件 provider-form.js）：ID（新建时写）、显示名、地址、接口、密钥（粘贴
// 或者环境变量）。存的先后：粘贴了密钥的先 secret.set 存成新名字，再一条 config.set 写别的几项和密钥的引用；config.set 拒了的，
// 已经存下的密钥名字写在报错后面。明文只在框里，存完、取消就扔。
import { element, say } from '../../shell/i18n.js';
import { openDialog } from '../../shell/dialog.js';
import { core, refusalText } from '../../shell/core.js';
import { envRef, inputText, providerChanges, providerName, secretName, validId } from './config.js';

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
  // 地址是环境变量引用的：框空着就是不改。
  const urlEnv = envRef(provider?.base_url);
  const urlField = input({
    value: urlEnv ? '' : inputText(provider?.base_url ?? ''),
    placeholder: urlEnv ? say('models/from-env', { name: urlEnv }) : 'https://…',
  });
  const drivers = state.schema?.items.find((one) => one.key === 'providers.<id>.driver')?.options ?? [];
  const driverField = element('select', 'select');
  for (const option of drivers) driverField.append(new Option(option.name ?? option.value, option.value));
  driverField.value = provider?.driver ?? drivers[0]?.value ?? '';
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
      base_url: urlEnv && !urlField.value.trim() ? null : urlField.value,
      driver: driverField.value,
      key: { kind: keyKind === 'secret' && !secret ? 'keep' : keyKind, value: envField.value },
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
  const keyRow = element('div', 'persona-field');
  keyRow.append(element('span', 'persona-label', say('models/form-key')), kinds, keyBox);
  dialog.body.append(
    row('models/form-id', idField),
    row('models/form-name', nameField),
    row('models/form-url', urlField),
    row('models/form-driver', driverField),
    keyRow,
    error,
  );
  dialog.foot.append(element('span', 'grow'), cancel, save);
  (editing ? nameField : idField).focus();
}
