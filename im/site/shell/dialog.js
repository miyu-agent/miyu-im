// 浮窗（施工 I-6 中，照样板 d-im.html 的 .layer、.dialog）：盖一层半透明的底，中间一个浮窗。小浮窗（openDialog）头上标题和关闭，
// 中间内容，底下按钮；会话的大浮窗（施工 I-7 上）自己摆里面，经 openLayer 盖底。点底、按 Esc、点关闭都关；关了从页面上拿掉。另有
// 一条短提示（toast），一会儿自己消失。
import { element, icon, say } from './i18n.js';

/** 把浮窗 `dialog` 盖着底放上页面。交回 close()；关了调 `onClose`。 */
export function openLayer(dialog, onClose) {
  const layer = element('div', 'layer');
  const scrim = element('div', 'scrim');
  layer.append(scrim, dialog);
  document.body.append(layer);
  requestAnimationFrame(() => layer.classList.add('open'));
  const keys = (event) => {
    if (event.key === 'Escape') close();
  };
  const close = () => {
    document.removeEventListener('keydown', keys);
    layer.classList.remove('open');
    setTimeout(() => layer.remove(), 180);
    onClose?.();
  };
  document.addEventListener('keydown', keys);
  scrim.addEventListener('click', close);
  return close;
}

/** 关闭按钮：点了调 `close`。 */
export function closeButton(close) {
  const shut = element('button', 'icon-button');
  shut.title = say('dialog/close');
  shut.append(icon('i-x'));
  shut.addEventListener('click', close);
  return shut;
}

/** 开一个小浮窗：标题 `title`。交回 { body, foot, setTitle(text), close() }；关了调 `onClose`。 */
export function openDialog(title, onClose) {
  const dialog = element('div', 'dialog');
  const head = element('div', 'd-head');
  const heading = element('h2', null, title);
  const steps = element('div', 'steps');
  steps.hidden = true;
  const body = element('div', 'd-body');
  const foot = element('div', 'd-foot');
  dialog.append(head, steps, body, foot);
  const close = openLayer(dialog, onClose);
  head.append(heading, closeButton(close));
  return {
    head,
    body,
    foot,
    setTitle(text) {
      heading.textContent = text;
    },
    /** 头上的进度条：一共 `count` 段，前 `done` 段亮着。 */
    setSteps(done, count) {
      steps.hidden = false;
      steps.replaceChildren(...Array.from({ length: count }, (_, n) => element('i', n < done ? 'on' : null)));
    },
    close,
  };
}

/** 一条短提示 `text`，两秒后自己消失。 */
export function toast(text) {
  const shown = element('div', 'toast', text);
  document.body.append(shown);
  requestAnimationFrame(() => shown.classList.add('show'));
  setTimeout(() => {
    shown.classList.remove('show');
    setTimeout(() => shown.remove(), 200);
  }, 2000);
}

/** 把 `text` 放进剪贴板，成了提示一句。 */
export async function copy(text) {
  try {
    await navigator.clipboard.writeText(text);
    toast(say('dialog/copied'));
  } catch {
    // 浏览器不让：字照样在页面上，人自己选。
  }
}
