// 连接设置（设计 01 第八节第 2 条，施工 I-6 中；样子照样板 d-im.html 的 conn-layer）：连接卡的齿轮打开。驱动声明的几格（QQ：
// 监听地址（施工 I-27）、反向 WebSocket 端口、验证 Token），下面一行要填到那边的地址（照这时格里的端口拼，能复制）。保存只交改了的：端口变了的、Token
// 填了的（已设的 Token 留空就不改，不把它的值拿到页面上）；经宿主的 connection.save 存。
// 连接表（施工 I-81 下中）：给了连接表里那一项（`entry`）的，格里照它填，存照它的端口经 connection.edit 改；连接不止一个的多一个
// 「删除这个连接」，点一次换成「再点一次删除」，再点才删（connection.remove）。没给的（连接表还没写过、原来那一个没设 Token）照旧。
import { element, say } from '../../shell/i18n.js';
import { copy, openDialog, toast } from '../../shell/dialog.js';
import { secretInput } from '../../shell/secret.js';
import { platform } from './platforms.js';
import { privacyNotice, proxyControl } from './telegram.js';

/** 给连接 `one`（机器人，还没连上的是空的）开连接设置：`request` 调宿主的方法；`entry` 是它在连接表里的那一项，`count` 是表里
 *  几个连接。 */
export async function openSettings(request, one, { entry = null, count = 0 } = {}) {
  let status;
  const dialog = openDialog(say('settings/title', { platform: platform(one?.platform || 'qq').name() }));
  try {
    status = await request('status');
  } catch {
    dialog.body.replaceChildren(element('p', 'failed', say('failed/network')));
    return;
  }
  const shown = platform(one?.platform || status.platform);
  // 格里照连接表的那一项填（施工 I-81 下中），没有的照 status 顶上那几格（头一个连接）。
  // Telegram（施工 I-183 下中）：连接表照机器人号认，不照端口。
  if (!entry && one && shown.settings.some((field) => field.kind === 'bot')) {
    entry = (status.connections || []).find((item) => (one.bot && item.bot === one.bot) || (!one.bot && item.index === one.index)) || null;
  }
  if (entry) status = { ...status, ...entry };
  const rows = [];
  if (one?.implementation) {
    rows.push(element('p', 'faint lead', say('settings/driver', { implementation: one.implementation, version: one.version })));
  }
  const inputs = new Map();
  // 密钥那几格的名字。
  const secrets = new Set();
  // 代理那几格（Telegram）：名字 → proxyControl。
  const proxies = new Map();
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
    if (field.kind === 'bot') {
      row.append(element('span', 'mono soft', one?.username ? `@${one.username} · ${one.bot}` : one?.bot || ''));
      rows.push(row);
      continue;
    }
    if (field.kind === 'proxy') {
      const proxy = proxyControl(status.proxy ?? '');
      proxies.set(field.name, proxy);
      row.append(proxy.box);
      rows.push(row);
      continue;
    }
    if (field.kind === 'privacy') {
      rows.push(...privacyRows(row, status, { request, one }));
      continue;
    }
    if (field.kind === 'address') {
      const copier = element('button', 'btn', say('dialog/copy'));
      copier.addEventListener('click', () => copy(address()));
      row.append(addressShown, copier);
    } else {
      // 密钥默认圆点遮着、眼睛按钮看原文（施工 I-47）；别的照原样一个输入框。
      const secret = field.kind === 'port' || field.kind === 'text' ? null : secretInput('input mono');
      const input = secret ? secret.input : element('input', field.kind === 'port' ? 'input num' : 'input mono');
      if (field.kind === 'port') {
        input.inputMode = 'numeric';
        input.value = String(status[field.name] ?? '');
        input.addEventListener('input', () => { addressShown.textContent = address(); });
      } else if (field.kind === 'text') {
        input.value = String(status[field.name] ?? '');
        input.spellcheck = false;
      } else {
        input.placeholder = status[field.name] === 'set' ? say('settings/token-set') : '';
        secrets.add(field.name);
      }
      inputs.set(field.name, input);
      row.append(secret ? secret.box : input);
    }
    rows.push(row);
  }
  if (shown.address) addressShown.textContent = address();
  const failed = element('p', 'failed');
  dialog.body.replaceChildren(...rows, failed);
  const cancel = element('button', 'btn ghost', say('dialog/cancel'));
  cancel.addEventListener('click', dialog.close);
  const foot = [element('span', 'grow'), cancel];
  if (entry && count > 1) foot.unshift(removal(request, entry, dialog, failed));
  const save = element('button', 'btn primary', say('dialog/save'));
  save.addEventListener('click', async () => {
    const values = {};
    for (const [name, input] of inputs) {
      // 密钥照填没填算改没改（点了眼睛以后输入框的类型是 text，不能照类型认）。
      const changed = secrets.has(name) ? input.value.trim() !== '' : input.value !== String(status[name]);
      if (changed) values[name] = input.value;
    }
    for (const [name, proxy] of proxies) {
      if (proxy.value() !== (status[name] ?? '')) values[name] = proxy.value();
    }
    if (Object.keys(values).length === 0) {
      dialog.close();
      return;
    }
    save.disabled = true;
    try {
      if (entry) await request('connection.edit', { ...where(entry), values });
      else await request('connection.save', { values });
      toast(say('dialog/saved'));
      dialog.close();
    } catch (reason) {
      failed.textContent = failure(reason, values);
      save.disabled = false;
    }
  });
  dialog.foot.replaceChildren(...foot, save);
}

/** 存不成的那一句：宿主说写错了的，地址改了的是地址不是 IP（施工 I-116），端口重了的是端口占用了（施工 I-177）。 */
function failure(reason, values) {
  if (reason === 'config_invalid') return say('failed/invalid');
  if (reason === 'bad_params' && 'host' in values) return say('failed/host-ip');
  if (reason === 'port_taken') return say('failed/port-taken');
  return say('failed/save');
}

/** 「删除这个连接」：点一次换成「再点一次删除」，再点才删，删了关浮窗。 */
function removal(request, entry, dialog, failed) {
  const button = element('button', 'btn ghost danger', say('settings/remove'));
  let armed = false;
  button.addEventListener('click', async () => {
    if (!armed) {
      armed = true;
      button.textContent = say('settings/remove-again');
      return;
    }
    button.disabled = true;
    try {
      await request('connection.remove', where(entry));
      toast(say('dialog/saved'));
      dialog.close();
    } catch {
      failed.textContent = say('failed/save');
      button.disabled = false;
    }
  });
  return button;
}

/** 连接表里的一项照什么认：Telegram 的照机器人号，OneBot 的照端口。 */
function where(entry) {
  if (entry.bot) return { bot: entry.bot };
  // 还没认出号的令牌表连接（令牌一开始就失效这类）照第几个。
  return entry.index !== undefined ? { index: entry.index } : { port: entry.listen };
}

/** 隐私模式那一行（Telegram）：开着、关着，能再查一次；开着的下面照做的几步。 */
function privacyRows(row, status, { request, one }) {
  const state = element('span', 'tg-privacy-state');
  const notice = privacyNotice();
  const draw = (on) => {
    state.replaceChildren(element('span', on ? 'dot warn' : 'dot on'), document.createTextNode(` ${say(on ? 'tg/privacy-on-short' : 'tg/privacy-off-short')}`));
    notice.hidden = !on;
  };
  const again = element('button', 'btn', say('tg/check-again'));
  again.addEventListener('click', async () => {
    again.disabled = true;
    try {
      draw((await request('connection.check', one?.bot ? { bot: one.bot } : { index: one?.index })).privacy);
    } catch {
      // 查不成的照旧。
    }
    again.disabled = false;
  });
  draw(Boolean(status.privacy));
  row.append(state, again);
  return [row, notice];
}
