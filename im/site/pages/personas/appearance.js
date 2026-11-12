// 人格的外观三行（设计 09，施工 I-146；照网页软件的 appearance.js）：人格详情最上面头像、主色、背景图，换、删都当场存（只带这一样的
// persona.set），不等「保存」。图本身就是按钮（2026-10-11 项目主人定，不要一排字的按钮）：
// - 头像：圆的，没有的画一个「添加图片」的记号（不写名字的第一个字，2026-10-20 项目主人），点它换，悬停写「更换」；有的右上角 × 移除。
//   不是正方形的先裁（crop.js）。
// - 主色：「自动」（照头像取的第一名，就是不存）、核心算好的候选色（seeds，最多四个，2026-10-14 项目主人定头像取色归核心）、自选的、
//   「+」（浏览器的取色器）；现在用的那一块画勾。
// - 背景图：16:9 的小图，没有的写「添加」；有的悬停出「更换」「焦点」（focus.js），右上角 ×；下面透明度、模糊两个滑杆，松手存；模糊
//   拖到 0 就是删掉（2026-10-21 项目主人：不要开关）。网页软件拖的时候整页当场跟着变；im 网页不铺人格的背景图，只跟着写数。
import { element, icon, say } from '../../shell/i18n.js';
import { core, refusalText } from '../../shell/core.js';
import { lucide } from '../../shell/lucide.js';
import { AVATAR_BYTES, AVATAR_SIDE, BACKGROUND_BYTES, BACKGROUND_SIDE, imageUrl, onImage, shrink, store } from './images.js';
import { cropAvatar } from './crop.js';
import { chooseFocus } from './focus.js';

/** 人格没设透明度时画多少。 */
const OPACITY_DEFAULT = 20;

/** 外观三行：`request` 调宿主，人格 `id`；存了调 `saved()`，要写一句（出错、清掉）调 `say(text)`。交回 { rows, set(got), dispose() }。 */
export function appearance(request, id, { saved, say: note }) {
  let look = {};
  const avatarBox = element('div', 'persona-look-row');
  const seedBox = element('div', 'persona-look-row persona-seeds');
  const backgroundBox = element('div', 'persona-look-col');
  const stop = onImage(() => {
    if (avatarBox.isConnected) paint();
  });

  const setPart = async (part, value) => {
    try {
      await core(request, 'persona.set', { persona: id, [part]: value });
      look = await core(request, 'persona.get', { persona: id });
      note('');
      paint();
      saved();
    } catch (error) {
      paint();
      note(refusalText(error));
    }
  };

  const changeImage = async (part, file) => {
    try {
      const source = part === 'avatar' ? await cropAvatar(file) : file;
      if (!source) return;
      const blob =
        part === 'avatar'
          ? await shrink(source, AVATAR_SIDE, { maxBytes: AVATAR_BYTES })
          : await shrink(source, BACKGROUND_SIDE, { fallback: 'image/jpeg', maxBytes: BACKGROUND_BYTES });
      await setPart(part, { blob: await store(request, blob, part) });
    } catch (error) {
      note(refusalText(error));
    }
  };

  const paintAvatar = () => {
    const url = imageUrl(request, 'avatar', id, look.avatar);
    const pick = filePicker((file) => changeImage('avatar', file));
    const inner = url ? image(url) : element('span', 'persona-pic-empty');
    if (!url) inner.append(lucide('image-plus', icon('i-plus')));
    avatarBox.replaceChildren(
      picture('avatar', inner, true, () => pick.click(), look.avatar ? () => setPart('avatar', { unset: true }) : null),
      pick,
    );
  };

  const paintSeed = () => {
    const colors = Array.isArray(look.seeds) ? look.seeds : [];
    const input = element('input', 'persona-color-input');
    input.type = 'color';
    input.value = look.seed ?? colors[0] ?? '#3368c0';
    input.tabIndex = -1;
    input.addEventListener('change', () => setPart('seed', input.value));
    const open = () => (typeof input.showPicker === 'function' ? input.showPicker() : input.click());
    const block = (title, on, fill, mark, act) => {
      const button = element('button', on ? 'persona-swatch on' : 'persona-swatch');
      button.title = title;
      if (fill) Object.assign(button.style, { background: fill, color: inkOf(fill) });
      if (on) button.append(icon('i-check'));
      else if (typeof mark === 'string') button.textContent = mark;
      else if (mark) button.append(mark);
      button.addEventListener('click', act);
      return button;
    };
    const own = look.seed && !colors.includes(look.seed) ? look.seed : null;
    seedBox.replaceChildren(
      block(say('personas/seed-auto'), !look.seed, colors[0] ?? null, 'A', () => look.seed && setPart('seed', { unset: true })),
      ...colors.map((hex) => block(hex, look.seed === hex, hex, null, () => look.seed !== hex && setPart('seed', hex))),
      ...(own ? [block(own, true, own, null, open)] : []),
      block(say('personas/seed-pick'), false, null, icon('i-plus'), open),
      input,
    );
  };

  const paintBackground = () => {
    const url = imageUrl(request, 'background', id, look.background);
    const pick = filePicker((file) => changeImage('background', file));
    let inner;
    if (url) {
      inner = image(url);
    } else {
      inner = element('span', 'persona-pic-empty');
      inner.append(lucide('image', icon('i-plus')), element('span', null, say('personas/add')));
    }
    const focus = async () => {
      const got = await chooseFocus(url, look.background_focus ?? null);
      if (got === 'center') await setPart('background_focus', { unset: true });
      else if (got) await setPart('background_focus', got);
    };
    const actions = url ? [{ label: 'personas/focus', act: focus }] : [];
    const remove = look.background ? () => setPart('background', { unset: true }) : null;
    backgroundBox.replaceChildren(picture('background', inner, Boolean(url), () => pick.click(), remove, actions), pick);
    if (url) {
      backgroundBox.append(
        slider('personas/opacity', look.background_opacity ?? OPACITY_DEFAULT, (value) => `${value}%`, (value) =>
          setPart('background_opacity', value),
        ),
        slider('personas/blur', look.background_blur ?? 0, String, (value) =>
          setPart('background_blur', value > 0 ? value : { unset: true }),
        ),
      );
    }
  };

  const paint = () => {
    paintAvatar();
    paintSeed();
    paintBackground();
  };

  return {
    rows: [
      ['personas/avatar', avatarBox],
      ['personas/seed', seedBox],
      ['personas/background', backgroundBox],
    ],
    set(got) {
      look = got;
      paint();
    },
    dispose: stop,
  };
}

/** 一张图：图本身是按钮，点了换；有图的悬停盖一层写「更换」；`remove` 有的右上角一个小 ×。`actions` 有的（背景图），悬停那一层里是一排
 *  按钮：「更换」和它们。 */
function picture(part, inner, hover, change, remove, actions = []) {
  const box = element('div', `persona-pic is-${part}`);
  let main;
  if (actions.length) {
    main = element('div', 'persona-pic-main');
    const layer = element('span', 'persona-pic-hover');
    for (const one of [{ label: 'personas/change', act: change }, ...actions]) {
      const button = element('button', 'persona-pic-action', say(one.label));
      button.addEventListener('click', one.act);
      layer.append(button);
    }
    main.append(inner, layer);
  } else {
    main = element('button', 'persona-pic-main');
    main.title = say(`personas/${part}-change`);
    main.addEventListener('click', change);
    main.append(inner);
    if (hover) main.append(element('span', 'persona-pic-hover', say('personas/change')));
  }
  box.append(main);
  if (remove) {
    const cross = element('button', 'persona-pic-remove');
    cross.title = say(`personas/${part}-remove`);
    cross.append(icon('i-x'));
    cross.addEventListener('click', remove);
    box.append(cross);
  }
  return box;
}

/** 一行滑杆：名字、0 到 100 的滑杆、写着的值（`shown` 写法）。拖的时候跟着写数，松手调 `commit`。 */
function slider(label, value, shown, commit) {
  const line = element('div', 'persona-slider');
  const text = element('span', 'persona-slider-value', shown(value));
  const range = element('input', 'persona-range');
  Object.assign(range, { type: 'range', min: '0', max: '100', step: '1', value: String(value) });
  range.setAttribute('aria-label', say(label));
  range.addEventListener('input', () => {
    text.textContent = shown(Number(range.value));
  });
  range.addEventListener('change', () => commit(Number(range.value)));
  line.append(element('span', 'persona-label', say(label)), range, text);
  return line;
}

/** 藏着的选图框，按钮点它。 */
function filePicker(onPick) {
  const pick = element('input');
  Object.assign(pick, { type: 'file', accept: 'image/png,image/jpeg,image/webp,image/*', hidden: true });
  pick.addEventListener('change', () => {
    const file = pick.files?.[0];
    pick.value = '';
    if (file) onPick(file);
  });
  return pick;
}

function image(url) {
  const shown = element('img');
  shown.src = url;
  shown.alt = '';
  return shown;
}

/** 铺在 `hex` 上的字用黑还是白（照亮度，纯逻辑）。 */
export function inkOf(hex) {
  const value = Number.parseInt(hex.slice(1), 16);
  const [r, g, b] = [(value >> 16) & 255, (value >> 8) & 255, value & 255];
  return 0.299 * r + 0.587 * g + 0.114 * b > 150 ? '#1b1b1f' : '#ffffff';
}
