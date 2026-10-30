// 新建连接（设计 01 第八节第 2 条，施工 I-6 中；样子照样板 d-im.html）：选一个平台 → 平台的几步（QQ：照着填地址「我已填写」、粘贴
// Token「保存并连接」）→ 等它连上（两个节点之间一条线、光点来回跑）→ 连上了画一个勾，写机器人叫什么、连上的是哪个实现。
// 存设置经宿主的 connection.save（人的连接替人存）；等的时候照外壳推来的连接看，有连着的就算连上。
// 连接表（施工 I-81 下中）：填地址那一步多一格端口，先填一个没用过的（表里设了 Token 的端口最大的加一，表里还没有的照 status 的
// 端口），和表里的重了不让往下走；粘贴 Token 那一步存成连接表里新的一个（connection.add）；等连上照这个端口认号。
import { element, icon, say } from '../../shell/i18n.js';
import { copy, openDialog } from '../../shell/dialog.js';
import { secretInput } from '../../shell/secret.js';
import { PLATFORMS, mark, platform } from './platforms.js';

/** 开新建连接的浮窗：`request` 调宿主的方法，`table` 是首页读到的连接表。交回 update(connections)（外壳推来的连接，等连上用）。 */
export function openWizard(request, table = { entries: [] }) {
  const used = table.entries.filter((entry) => entry.token === 'set').map((entry) => entry.listen);
  // 新连接的端口：填地址那一步定。
  let port = used.length ? Math.max(...used) + 1 : null;
  // 先选着第一个平台（现在只有 QQ），进度条照它的步数画。
  let chosen = Object.keys(PLATFORMS)[0] ?? null;
  let at = 0;
  let waiting = false;
  let latest = [];
  // 连上了画着的那一个：名字后到的（页面问了宿主才去问平台），到了再画一遍。
  let shownUp = null;
  const dialog = openDialog(say('wizard/pick'), () => { waiting = false; });
  const steps = () => (chosen ? platform(chosen).steps : []);
  const total = () => 1 + steps().length + 1;
  // 中间换成第 `to` 屏（选平台 0、平台的几步、等连上、连上了）：往前翻的从右边滑进来、往回翻的从左边（施工 I-47）；头一屏随浮窗
  // 一起出来，同一屏重画的不动。
  let shownAt = null;
  const show = (to, ...children) => {
    if (shownAt === null || shownAt === to) dialog.body.replaceChildren(...children);
    else dialog.swap(children, to < shownAt);
    shownAt = to;
  };

  const buttons = (back, next, label, enabled = true) => {
    const backButton = element('button', 'btn ghost', say('wizard/back'));
    backButton.hidden = !back;
    backButton.addEventListener('click', back || (() => {}));
    const nextButton = element('button', 'btn primary', label);
    nextButton.disabled = !enabled;
    nextButton.addEventListener('click', next);
    dialog.foot.replaceChildren(backButton, element('span', 'grow'), nextButton);
    return nextButton;
  };

  const pick = () => {
    at = 0;
    dialog.setTitle(say('wizard/pick'));
    dialog.setSteps(1, total());
    const list = element('div', 'platforms');
    for (const [id, shown] of Object.entries(PLATFORMS)) {
      const choice = element('button', `pf${chosen === id ? ' on' : ''}`);
      const tile = element('span', `pi pf-${id}`);
      tile.append(mark(shown));
      choice.append(tile, element('b', null, shown.name()), element('small', 'faint', shown.driver()));
      choice.addEventListener('click', () => {
        chosen = id;
        pick();
      });
      list.append(choice);
    }
    show(0, list);
    buttons(null, () => step(1), say('wizard/next'), Boolean(chosen));
  };

  const step = async (n) => {
    at = n;
    if (n > steps().length) {
      wait();
      return;
    }
    const shown = steps()[n - 1];
    dialog.setTitle(shown.title());
    dialog.setSteps(n + 1, total());
    const text = element('p', 'soft lead', shown.text());
    const back = () => (n === 1 ? pick() : step(n - 1));
    if (shown.kind === 'address') {
      const portFill = element('div', 'fill');
      const portInput = element('input', 'input num');
      portInput.inputMode = 'numeric';
      portFill.append(element('span', 'soft', say('wizard/port')), portInput, element('span'));
      const fill = element('div', 'fill');
      const code = element('code', null, '…');
      const copier = element('button', 'btn', say('dialog/copy'));
      fill.append(element('span', 'soft', say('wizard/address')), code, copier);
      const failed = element('p', 'failed');
      show(n, text, portFill, fill, failed);
      const next = buttons(back, () => step(n + 1), shown.next(), false);
      let status;
      try {
        status = await request('status');
      } catch {
        code.textContent = say('failed/network');
        return;
      }
      if (port === null) port = status.listen;
      portInput.value = String(port);
      const address = () => platform(chosen).address({ ...status, listen: port });
      const check = () => {
        const typed = Number(portInput.value);
        const fine = Number.isInteger(typed) && typed >= 1024 && typed <= 65535;
        const taken = used.includes(typed);
        failed.textContent = taken ? say('failed/port-taken') : '';
        if (fine) port = typed;
        code.textContent = address();
        next.disabled = !fine || taken;
      };
      portInput.addEventListener('input', check);
      copier.addEventListener('click', () => copy(address()));
      check();
      return;
    }
    // 粘贴一个密钥，存了就连。
    const fill = element('div', 'fill');
    // 密钥默认圆点遮着、眼睛按钮看原文（施工 I-47）。
    const { box, input } = secretInput('input mono');
    input.placeholder = shown.placeholder();
    fill.append(element('span', 'soft', shown.label()), box, element('span'));
    const failed = element('p', 'failed');
    show(n, text, fill, failed);
    const next = buttons(back, async () => {
      next.disabled = true;
      try {
        await request('connection.add', { port, [shown.name]: input.value });
        step(n + 1);
      } catch (reason) {
        if (reason === 'config_invalid') failed.textContent = say('failed/invalid');
        else if (reason === 'bad_params') failed.textContent = say('failed/port-taken');
        else failed.textContent = say('failed/save');
        next.disabled = false;
      }
    }, shown.next(), false);
    input.addEventListener('input', () => { next.disabled = input.value.trim() === ''; });
    input.focus();
  };

  const wait = () => {
    waiting = true;
    const shown = platform(chosen);
    dialog.setTitle(say('wizard/wait-title'));
    dialog.setSteps(total(), total());
    const link = element('div', 'link');
    const ours = element('div', 'node-wrap');
    const us = element('span', 'node node-us');
    us.append(icon('i-link'));
    ours.append(us, document.createTextNode(say('title')));
    const wire = element('div', 'wire');
    wire.append(element('i'));
    const theirs = element('div', 'node-wrap');
    const node = element('span', `node pf-text-${chosen}`);
    node.append(mark(shown));
    theirs.append(node, document.createTextNode(shown.node()));
    link.append(ours, wire, theirs);
    const box = element('div', 'center');
    box.append(link, element('p', 'soft', shown.waiting()));
    show(steps().length + 1, box);
    buttons(() => step(steps().length), () => {}, say('wizard/next'), false).hidden = true;
    check();
  };

  const check = () => {
    if (shownUp && !shownUp.name) {
      const named = latest.find((one) => one.bot === shownUp.bot && one.name);
      if (named) done(named);
      return;
    }
    const up = latest.find((one) => one.connected && one.platform === chosen && (port === null || one.listen === port));
    if (!waiting || !up) return;
    waiting = false;
    done(up);
  };

  const done = (up) => {
    shownUp = up;
    const shown = platform(chosen);
    dialog.setTitle(say('wizard/done-title'));
    const mark = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
    mark.setAttribute('class', 'done-mark');
    mark.setAttribute('viewBox', '0 0 64 64');
    mark.innerHTML = '<circle cx="32" cy="32" r="30"/><path d="M20 33l8 8 16-17"/>';
    const box = element('div', 'center');
    const detail = up.implementation
      ? say('wizard/done-detail', { bot: up.bot, implementation: up.implementation, version: up.version })
      : say('wizard/done-bot', { bot: up.bot });
    const line = up.name
      ? say('wizard/done', { name: up.name, platform: shown.name() })
      : say('wizard/done-unnamed', { platform: shown.name() });
    box.append(mark, element('p', 'done-line', line), element('p', 'soft', detail));
    show(steps().length + 2, box);
    const finish = element('button', 'btn primary', say('wizard/finish'));
    finish.addEventListener('click', dialog.close);
    dialog.foot.replaceChildren(element('span', 'grow'), finish);
  };

  pick();
  return {
    update(connections) {
      latest = connections;
      check();
    },
  };
}
