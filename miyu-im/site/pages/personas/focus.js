// 选背景图的焦点（设计 09，施工 I-146；照网页软件的 focus.js，2026-10-11 项目主人定：背景图不裁、只选焦点）：浮窗里整张图照比例
// 放进框，点一下或拖那个圆点定焦点；下面两个小框照窄的、宽的窗口画出会露哪一块。「居中」删掉焦点、「取消」不动、「确定」存。
// 网页软件拖的时候整页当场跟着变；im 网页不铺人格的背景图，只看这两个小框。
import { element, say } from '../../shell/i18n.js';
import { openDialog } from '../../shell/dialog.js';

/** 框最大多大（CSS 像素）。 */
const BOX_W = 440;
const BOX_H = 260;

/** 指针落在图上哪一点：百分数、整数、夹在 0 到 100（纯逻辑）。 */
export function pointAt(rect, x, y) {
  const percent = (value, from, size) => Math.round(Math.min(100, Math.max(0, ((value - from) / Math.max(1, size)) * 100)));
  return [percent(x, rect.left, rect.width), percent(y, rect.top, rect.height)];
}

/** 图照比例放进框里多大（纯逻辑）。 */
export function fitBox(picture) {
  const scale = Math.min(BOX_W / picture.w, BOX_H / picture.h);
  return { w: Math.round(picture.w * scale), h: Math.round(picture.h * scale) };
}

/** 开浮窗选焦点：`url` 是背景图，`focus` 是现在的（没设的是 null，居中）。交回选定的 [x, y]、'center'（居中）、null（取消）。 */
export async function chooseFocus(url, focus) {
  const image = new Image();
  image.src = url;
  await image.decode();
  const size = fitBox({ w: image.naturalWidth, h: image.naturalHeight });
  return new Promise((resolve) => {
    let at = focus ?? [50, 50];
    let done = false;
    const finish = (result) => {
      if (done) return;
      done = true;
      resolve(result);
      dialog.close();
    };
    const dialog = openDialog(say('personas/focus-title'), () => finish(null));
    const dot = element('span', 'persona-focus-dot');
    const picture = element('div', 'persona-focus-picture');
    Object.assign(picture.style, { width: `${size.w}px`, height: `${size.h}px`, backgroundImage: `url("${url}")` });
    picture.append(dot);
    const sample = (kind, label) => {
      const figure = element('figure', 'persona-focus-sample');
      const frame = element('span', `persona-focus-frame ${kind}`);
      frame.style.backgroundImage = `url("${url}")`;
      figure.append(frame, element('figcaption', null, say(label)));
      return figure;
    };
    const narrow = sample('narrow', 'personas/focus-narrow');
    const wide = sample('wide', 'personas/focus-wide');
    const draw = () => {
      dot.style.left = `${at[0]}%`;
      dot.style.top = `${at[1]}%`;
      for (const figure of [narrow, wide]) figure.firstChild.style.backgroundPosition = `${at[0]}% ${at[1]}%`;
    };
    let drag = null;
    const move = (event) => {
      at = pointAt(picture.getBoundingClientRect(), event.clientX, event.clientY);
      draw();
    };
    picture.addEventListener('pointerdown', (event) => {
      drag = event.pointerId;
      picture.setPointerCapture(event.pointerId);
      move(event);
    });
    picture.addEventListener('pointermove', (event) => {
      if (drag === event.pointerId) move(event);
    });
    const drop = () => {
      drag = null;
    };
    picture.addEventListener('pointerup', drop);
    picture.addEventListener('pointercancel', drop);
    const button = (text, className, act) => {
      const shown = element('button', className, say(text));
      shown.addEventListener('click', act);
      return shown;
    };
    const samples = element('div', 'persona-focus-samples');
    samples.append(narrow, wide);
    dialog.body.classList.add('persona-focus');
    dialog.body.append(picture, samples);
    dialog.foot.append(
      button('personas/focus-center', 'btn ghost', () => finish('center')),
      element('span', 'grow'),
      button('dialog/cancel', 'btn', () => finish(null)),
      button('personas/ok', 'btn primary', () => finish(at)),
    );
    draw();
  });
}
