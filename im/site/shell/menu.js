// 下拉（施工 I-272，照网页包设置页的下拉）：页面上一个像下拉的按钮，点了在它下面弹一张小卡片，一项一行，选着的那一项后面打 ✓。
// 换掉原生 <select>（浏览器自带的粗箭头、系统弹层，和网页包两个样子）。
//
// - 用法和原生的一样：交回的按钮有 `.value`（读、写都行）、`.disabled`，选了别的一项发一声 `change`；`setOptions` 换一套选项。
// - 选项：[{ value, text, sub?, off? }]，`value` 是字（同原生 <option> 的 value）；`sub` 是名字后面的浅灰小字；`off` 的暗着、点不了。
// - 键盘：按钮上回车、空格、上下键打开；卡片里上下键挪、回车选、Esc 关；点卡片外面关。
import { element, icon } from './i18n.js';

/** 一个下拉：`options` 见上，`value` 是一开始选着的，`className` 另加在按钮上。 */
export function menuSelect(options, value, className) {
  const button = element('button', className ? `pk-trigger mn-trigger ${className}` : 'pk-trigger mn-trigger');
  button.type = 'button';
  const label = element('span', 'pk-label');
  button.append(label, icon('i-down'));
  let items = [];
  let current = null;

  const draw = () => {
    const chosen = items.find((one) => one.value === current);
    label.textContent = chosen ? chosen.text : '';
    button.title = chosen ? chosen.text : '';
  };
  button.setOptions = (next) => {
    items = (next || []).map((one) => ({ value: String(one.value), text: one.text, sub: one.sub, off: Boolean(one.off) }));
    if (!items.some((one) => one.value === current)) current = items[0]?.value ?? null;
    draw();
  };
  Object.defineProperty(button, 'value', {
    get: () => current ?? '',
    set: (next) => {
      current = next == null ? null : String(next);
      draw();
    },
  });
  Object.defineProperty(button, 'options', { get: () => items });

  button.setOptions(options);
  if (value != null) button.value = value;

  button.addEventListener('click', () => open(button, items, current, (picked) => {
    if (picked === current) return;
    current = picked;
    draw();
    button.dispatchEvent(new Event('change', { bubbles: true }));
  }));
  button.addEventListener('keydown', (event) => {
    if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return;
    event.preventDefault();
    button.click();
  });
  return button;
}

/** 在 `anchor` 下面弹卡片；选了调 `pick(值)`。 */
function open(anchor, items, current, pick) {
  if (items.length === 0) return;
  const pop = element('div', 'mn-pop');
  pop.setAttribute('role', 'listbox');
  const rows = items.map((one) => {
    const row = element('button', one.value === current ? 'mn-item on' : 'mn-item');
    row.type = 'button';
    row.disabled = one.off && one.value !== current;
    row.setAttribute('role', 'option');
    const text = element('span', 'mn-text', one.text);
    if (one.sub) text.append(element('em', null, one.sub));
    row.append(text);
    if (one.value === current) row.append(icon('i-check'));
    row.addEventListener('click', () => {
      close();
      pick(one.value);
    });
    return row;
  });
  pop.append(...rows);
  document.body.append(pop);
  place(anchor, pop);

  const outside = (event) => {
    if (!pop.contains(event.target) && !anchor.contains(event.target)) close();
  };
  const keys = (event) => {
    if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      close();
      anchor.focus();
    } else if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      const step = event.key === 'ArrowDown' ? 1 : -1;
      const live = rows.filter((row) => !row.disabled);
      const from = live.indexOf(document.activeElement);
      live[(from + step + live.length) % live.length]?.focus();
    } else if (event.key === 'Tab') close();
  };
  const away = () => close();
  function close() {
    pop.remove();
    document.removeEventListener('pointerdown', outside, true);
    document.removeEventListener('keydown', keys, true);
    window.removeEventListener('resize', away);
    document.removeEventListener('scroll', away, true);
  }
  // 这一下点击还在冒泡：下一拍再听外面的点击。
  setTimeout(() => {
    document.addEventListener('pointerdown', outside, true);
    window.addEventListener('resize', away);
    document.addEventListener('scroll', (event) => {
      if (!pop.contains(event.target)) away();
    }, true);
  });
  document.addEventListener('keydown', keys, true);
  (rows.find((row) => row.classList.contains('on')) || rows[0]).focus();
}

/** 卡片放在按钮下面，左边对齐；下面放不下翻到上面，右边出界往左挪。 */
function place(anchor, pop) {
  const box = anchor.getBoundingClientRect();
  pop.style.minWidth = `${Math.max(box.width, 160)}px`;
  const width = pop.offsetWidth;
  const height = pop.offsetHeight;
  const below = window.innerHeight - box.bottom - 8;
  const top = below >= height || box.top < height ? box.bottom + 4 : box.top - height - 4;
  const left = Math.max(8, Math.min(box.left, window.innerWidth - width - 8));
  pop.style.top = `${Math.max(8, top)}px`;
  pop.style.left = `${left}px`;
}
