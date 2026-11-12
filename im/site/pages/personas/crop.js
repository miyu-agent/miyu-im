// 裁头像（设计 09，施工 I-146；照网页软件的 crop.js，2026-10-11 项目主人定：比例不对才弹）：选的图不是正方形（长宽差过 2%）先弹一个
// 浮窗：图上一个圆的框（框外压暗），拖图挪位置、滑杆或滚轮放大，「取消」「确定」；确定了裁成正方形交回。正方形的直接交回原图。
import { element, say } from '../../shell/i18n.js';
import { openDialog } from '../../shell/dialog.js';

/** 框的边长（CSS 像素）。 */
const VIEW = 280;
/** 最多放大到铺满框的几倍。 */
const MOST_ZOOM = 4;
/** 长宽差多少以内算正方形。 */
const SQUARE = 0.02;
/** 裁出来最大多大（再交给 images.js 的 shrink 缩到存的大小）。 */
const OUT = 1024;

/** 放大 `zoom` 倍（1 是刚好铺满框）时图画多大的比例（纯逻辑）。 */
export function coverScale(picture, zoom) {
  return (VIEW / Math.min(picture.w, picture.h)) * zoom;
}

/** 夹住位置：图不能挪到露出框外的空（纯逻辑）。 */
export function clampAt(picture, scale, x, y) {
  return { x: Math.min(0, Math.max(VIEW - picture.w * scale, x)), y: Math.min(0, Math.max(VIEW - picture.h * scale, y)) };
}

/** 是不是（差不多）正方形（纯逻辑）。 */
export const isSquare = (picture) => Math.abs(picture.w - picture.h) / Math.max(picture.w, picture.h) <= SQUARE;

/** 弹窗裁 `file`：交回裁好的、原图（正方形的）；取消的是 null。 */
export async function cropAvatar(file) {
  const bitmap = await createImageBitmap(file);
  const picture = { w: bitmap.width, h: bitmap.height };
  if (isSquare(picture)) {
    bitmap.close();
    return file;
  }
  return new Promise((resolve) => {
    let done = false;
    const finish = (blob) => {
      if (done) return;
      done = true;
      bitmap.close();
      resolve(blob);
    };
    const dialog = openDialog(say('personas/crop-title'), () => finish(null));
    const ratio = Math.max(1, window.devicePixelRatio || 1);
    const canvas = element('canvas', 'persona-crop-canvas');
    canvas.width = Math.round(VIEW * ratio);
    canvas.height = Math.round(VIEW * ratio);
    const paint = canvas.getContext('2d');
    let zoom = 1;
    let scale = coverScale(picture, zoom);
    let at = clampAt(picture, scale, (VIEW - picture.w * scale) / 2, (VIEW - picture.h * scale) / 2);
    const draw = () => {
      paint.setTransform(ratio, 0, 0, ratio, 0, 0);
      paint.clearRect(0, 0, VIEW, VIEW);
      paint.drawImage(bitmap, at.x, at.y, picture.w * scale, picture.h * scale);
    };
    const slider = element('input', 'persona-range');
    Object.assign(slider, { type: 'range', min: '1', max: String(MOST_ZOOM), step: '0.01', value: '1' });
    slider.setAttribute('aria-label', say('personas/crop-zoom'));
    // 放大到 `next`，框的正中对着的那一点不动。
    const zoomTo = (next) => {
      zoom = Math.min(MOST_ZOOM, Math.max(1, next));
      const before = scale;
      scale = coverScale(picture, zoom);
      const middle = VIEW / 2;
      at = clampAt(picture, scale, middle - ((middle - at.x) / before) * scale, middle - ((middle - at.y) / before) * scale);
      slider.value = String(zoom);
      draw();
    };
    slider.addEventListener('input', () => zoomTo(Number(slider.value)));
    const stage = element('div', 'persona-crop-stage');
    stage.append(canvas, element('div', 'persona-crop-ring'));
    let drag = null;
    stage.addEventListener('pointerdown', (event) => {
      drag = { id: event.pointerId, x: event.clientX, y: event.clientY };
      stage.setPointerCapture(event.pointerId);
    });
    stage.addEventListener('pointermove', (event) => {
      if (!drag || drag.id !== event.pointerId) return;
      at = clampAt(picture, scale, at.x + event.clientX - drag.x, at.y + event.clientY - drag.y);
      drag = { id: event.pointerId, x: event.clientX, y: event.clientY };
      draw();
    });
    const drop = () => {
      drag = null;
    };
    stage.addEventListener('pointerup', drop);
    stage.addEventListener('pointercancel', drop);
    stage.addEventListener(
      'wheel',
      (event) => {
        event.preventDefault();
        zoomTo(zoom * Math.exp(-event.deltaY * 0.0015));
      },
      { passive: false },
    );
    const ok = element('button', 'btn primary', say('personas/ok'));
    const cancel = element('button', 'btn', say('dialog/cancel'));
    cancel.addEventListener('click', () => dialog.close());
    ok.addEventListener('click', () => {
      const side = VIEW / scale;
      const out = Math.max(1, Math.min(OUT, Math.round(side)));
      const cut = document.createElement('canvas');
      cut.width = out;
      cut.height = out;
      cut.getContext('2d').drawImage(bitmap, -at.x / scale, -at.y / scale, side, side, 0, 0, out, out);
      cut.toBlob((blob) => {
        finish(blob);
        dialog.close();
      }, 'image/png');
    });
    dialog.body.classList.add('persona-crop');
    dialog.body.append(stage, slider);
    dialog.foot.append(element('span', 'grow'), cancel, ok);
    draw();
  });
}
