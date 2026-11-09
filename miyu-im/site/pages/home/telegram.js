// Telegram 的几块（施工 I-183 下中，设计 07 第 3.5 节）：新建连接、连接设置共用。
//
// - 代理（每个连接一份）：跟随 Miyu 的联网设置（存 `env`，空的也认：核心的一列字不收空字）、直连（存 `direct`）、自定义（存地址）。
// - 查的结果：宿主照 getMe 答的 `{bot, name, username, privacy}`，`privacy` 是隐私模式开着（getMe 的 can_read_all_group_messages 为假）。
// - 隐私模式开着时的提醒：群里只收得到回复它的、发给它的命令，@ 它也收不到；关掉以后要把机器人移出群再拉回来。
import { element, icon, say } from '../../shell/i18n.js';

/** 代理那一格：交回 { box, value() }。`given` 是存着的值。 */
export function proxyControl(given = 'env') {
  let mode = given === '' || given === 'env' ? 'env' : given === 'direct' ? 'direct' : 'custom';
  const box = element('div', 'tg-proxy');
  const seg = element('div', 'seg');
  const input = element('input', 'input mono');
  input.spellcheck = false;
  input.placeholder = say('settings/tg-proxy-placeholder');
  if (mode === 'custom') input.value = given;
  const draw = () => {
    seg.replaceChildren();
    for (const [value, label] of [['env', say('settings/tg-proxy-env')], ['direct', say('settings/tg-proxy-direct')], ['custom', say('settings/tg-proxy-custom')]]) {
      const button = element('button', value === mode ? 'on' : null, label);
      button.type = 'button';
      button.addEventListener('click', () => {
        mode = value;
        draw();
        if (mode === 'custom') input.focus();
      });
      seg.append(button);
    }
    input.hidden = mode !== 'custom';
  };
  draw();
  box.append(seg, input);
  return { box, value: () => (mode === 'env' ? 'env' : mode === 'direct' ? 'direct' : input.value.trim() || 'env') };
}

/** 查的结果一行：`ok` 的打勾，不然画提醒。 */
function checkRow(ok, text, detail) {
  const row = element('div', ok ? 'tg-check ok' : 'tg-check warn');
  row.append(icon(ok ? 'i-check' : 'i-eye-off'));
  const words = element('div', 'grow');
  words.append(element('b', null, text));
  if (detail) words.append(element('small', 'faint', detail));
  row.append(words);
  return row;
}

/** 新建连接查完以后那一屏的几行：令牌、连得上、隐私模式。 */
export function checkRows(probe) {
  const box = element('div', 'tg-checks');
  const who = probe.username ? `@${probe.username}` : probe.bot;
  box.append(
    checkRow(true, say('tg/token-ok'), say('tg/token-ok-detail', { who, name: probe.name || who, bot: probe.bot })),
    checkRow(true, say('tg/reach-ok')),
    probe.privacy ? checkRow(false, say('tg/privacy-on')) : checkRow(true, say('tg/privacy-off')),
  );
  if (probe.privacy) box.append(privacyNotice());
  return box;
}

/** 隐私模式开着：照做的几步。 */
export function privacyNotice() {
  const notice = element('div', 'plug-notice tg-privacy');
  notice.append(element('p', null, say('tg/privacy-why')));
  const steps = element('ol');
  for (const key of ['tg/privacy-step-1', 'tg/privacy-step-2', 'tg/privacy-step-3']) steps.append(element('li', null, say(key)));
  notice.append(steps, element('p', 'faint', say('tg/privacy-admin')));
  return notice;
}
