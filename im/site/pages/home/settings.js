// 连接设置（设计 01 第八节第 2 条，施工 I-6 中；样子照样板 d-im.html 的 conn-layer）：连接卡的齿轮打开。驱动声明的几格（QQ：
// 反向 WebSocket 端口、验证 Token），下面一行要填到那边的地址（照这时格里的端口拼，能复制）。保存只交改了的：端口变了的、Token
// 填了的（已设的 Token 留空就不改，不把它的值拿到页面上）；经宿主的 connection.save 存。
import { element, say } from '../../shell/i18n.js';
import { copy, openDialog, toast } from '../../shell/dialog.js';
import { platform } from './platforms.js';

/** 给连接 `one` 开连接设置：`request` 调宿主的方法。 */
export async function openSettings(request, one) {
  const shown = platform(one.platform);
  const dialog = openDialog(say('settings/title', { platform: shown.name() }));
  let status;
  try {
    status = await request('status');
  } catch {
    dialog.body.replaceChildren(element('p', 'failed', say('failed/network')));
    return;
  }
  const rows = [];
  if (one.implementation) {
    rows.push(element('p', 'faint lead', say('settings/driver', { implementation: one.implementation, version: one.version })));
  }
  const inputs = new Map();
  const addressShown = element('span', 'mono soft');
  const address = () => {
    const port = inputs.get('listen')?.value || status.listen;
    return shown.address({ ...status, listen: port });
  };
  for (const field of shown.settings) {
    const row = element('div', 'row');
    const label = element('div', 'label', field.label());
    if (field.hint) label.append(element('small', null, field.hint()));
    row.append(label);
    if (field.kind === 'address') {
      const copier = element('button', 'btn', say('dialog/copy'));
      copier.addEventListener('click', () => copy(address()));
      row.append(addressShown, copier);
    } else {
      const input = element('input', field.kind === 'port' ? 'input num' : 'input mono');
      if (field.kind === 'port') {
        input.inputMode = 'numeric';
        input.value = String(status[field.name] ?? '');
        input.addEventListener('input', () => { addressShown.textContent = address(); });
      } else {
        input.type = 'password';
        input.autocomplete = 'off';
        input.placeholder = status[field.name] === 'set' ? say('settings/token-set') : '';
      }
      inputs.set(field.name, input);
      row.append(input);
    }
    rows.push(row);
  }
  addressShown.textContent = address();
  const failed = element('p', 'failed');
  dialog.body.replaceChildren(...rows, failed);
  const cancel = element('button', 'btn ghost', say('dialog/cancel'));
  cancel.addEventListener('click', dialog.close);
  const save = element('button', 'btn primary', say('dialog/save'));
  save.addEventListener('click', async () => {
    const values = {};
    for (const [name, input] of inputs) {
      const changed = input.type === 'password' ? input.value.trim() !== '' : input.value !== String(status[name]);
      if (changed) values[name] = input.value;
    }
    if (Object.keys(values).length === 0) {
      dialog.close();
      return;
    }
    save.disabled = true;
    try {
      await request('connection.save', { values });
      toast(say('dialog/saved'));
      dialog.close();
    } catch (reason) {
      failed.textContent = reason === 'config_invalid' ? say('failed/invalid') : say('failed/save');
      save.disabled = false;
    }
  });
  dialog.foot.replaceChildren(element('span', 'grow'), cancel, save);
}
