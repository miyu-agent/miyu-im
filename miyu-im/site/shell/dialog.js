// 浮窗（施工 I-6 中，照样板 d-im.html 的 .layer、.dialog）：盖一层半透明的底，中间一个浮窗。小浮窗（openDialog）头上标题和关闭，
// 中间内容，底下按钮；会话的大浮窗（施工 I-7 上）自己摆里面，经 openLayer 盖底。点底、按 Esc、点关闭都关；关了从页面上拿掉。另有
// 一条短提示（toast），一会儿自己消失。
import { element, icon, say } from './i18n.js';

/** 开着的几层（后开的在后）：Esc 只关最上面那一层（会话浮窗上再开选模型的小窗时，Esc 只关小窗）。 */
const layers = [];

/** 把浮窗 `dialog` 盖着底放上页面。交回 close()；关了调 `onClose`。 */
export function openLayer(dialog, onClose) {
  const layer = element('div', 'layer');
  const scrim = element('div', 'scrim');
  layer.append(scrim, dialog);
  document.body.append(layer);
  // 先让浏览器算一遍起点的样子再加 `open`（施工 I-47）：等下一帧再加，常常和起点算在同一次里，过渡被跳过，浮窗一下子冒出来。
  layer.getBoundingClientRect();
  layer.classList.add('open');
  layers.push(layer);
  const keys = (event) => {
    if (event.key === 'Escape' && layers[layers.length - 1] === layer) {
      event.stopImmediatePropagation();
      close();
    }
  };
  const close = () => {
    if (!layers.includes(layer)) return;
    layers.splice(layers.indexOf(layer), 1);
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

/** 系统要求少动画的不做过渡。 */
const still = () => window.matchMedia('(prefers-reduced-motion: reduce)').matches;

/** 开一个小浮窗：标题 `title`。交回 { body, foot, setTitle(text), swap(children, back), close() }；关了调 `onClose`。 */
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
    /**
     * 换掉中间的内容（施工 I-47，项目主人 10-27 验收：上一步、下一步没有过渡）：新内容从右边滑进来，`back` 的从左边；浮窗的高度
     * 跟着变过去，不跳。
     */
    swap(children, back = false) {
      const from = dialog.getBoundingClientRect().height;
      body.replaceChildren(...children);
      if (still()) return;
      const to = dialog.getBoundingClientRect().height;
      const timing = { duration: 220, easing: 'cubic-bezier(0.22, 0.61, 0.36, 1)' };
      body.animate([{ opacity: 0, transform: `translateX(${back ? -16 : 16}px)` }, { opacity: 1, transform: 'none' }], timing);
      if (Math.abs(from - to) > 1) dialog.animate([{ height: `${from}px` }, { height: `${to}px` }], timing);
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
