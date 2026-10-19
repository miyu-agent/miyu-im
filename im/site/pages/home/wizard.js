// 新建连接（设计 01 第八节第 2 条，施工 I-6 中；样子照样板 d-im.html）：选一个平台 → 平台的几步（QQ：照着填地址「我已填写」、粘贴
// Token「保存并连接」）→ 等它连上（两个节点之间一条线、光点来回跑）→ 连上了画一个勾，写机器人叫什么、连上的是哪个实现。
// 存设置经宿主的 connection.save（人的连接替人存）；等的时候照外壳推来的连接看，有连着的就算连上。
import { element, icon, say } from '../../shell/i18n.js';
import { copy, openDialog } from '../../shell/dialog.js';
import { PLATFORMS, platform } from './platforms.js';

/** 开新建连接的浮窗：`request` 调宿主的方法。交回 update(connections)（外壳推来的连接，等连上用）。 */
export function openWizard(request) {
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
      choice.append(element('span', `pi pf-${id}`, shown.badge), element('b', null, shown.name()), element('small', 'faint', shown.driver()));
      choice.addEventListener('click', () => {
        chosen = id;
        pick();
      });
      list.append(choice);
    }
    dialog.body.replaceChildren(list);
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
      const fill = element('div', 'fill');
      const code = element('code', null, '…');
      const copier = element('button', 'btn', say('dialog/copy'));
      fill.append(element('span', 'soft', say('wizard/address')), code, copier);
      dialog.body.replaceChildren(text, fill);
      buttons(back, () => step(n + 1), shown.next());
      try {
        const address = platform(chosen).address(await request('status'));
        code.textContent = address;
        copier.addEventListener('click', () => copy(address));
      } catch {
        code.textContent = say('failed/network');
      }
      return;
    }
    // 粘贴一个密钥，存了就连。
    const fill = element('div', 'fill');
    const input = element('input', 'input mono');
    input.placeholder = shown.placeholder();
    input.autocomplete = 'off';
    fill.append(element('span', 'soft', shown.label()), input, element('span'));
    const failed = element('p', 'failed');
    dialog.body.replaceChildren(text, fill, failed);
    const next = buttons(back, async () => {
      next.disabled = true;
      try {
        await request('connection.save', { values: { [shown.name]: input.value } });
        step(n + 1);
      } catch (reason) {
        failed.textContent = reason === 'config_invalid' ? say('failed/invalid') : say('failed/save');
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
    theirs.append(element('span', `node pf-text-${chosen}`, shown.badge), document.createTextNode(shown.node()));
    link.append(ours, wire, theirs);
    const box = element('div', 'center');
    box.append(link, element('p', 'soft', shown.waiting()));
    dialog.body.replaceChildren(box);
    buttons(() => step(steps().length), () => {}, say('wizard/next'), false).hidden = true;
    check();
  };

  const check = () => {
    if (shownUp && !shownUp.name) {
      const named = latest.find((one) => one.bot === shownUp.bot && one.name);
      if (named) done(named);
      return;
    }
    const up = latest.find((one) => one.connected && one.platform === chosen);
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
    dialog.body.replaceChildren(box);
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
