// 新绑定一个本人的平台 ID（施工 I-111，项目主人 11-02：新建的时候给一个弹窗，第一步选平台，第二步填 ID）：浮窗照新建连接的样子
// （shell/dialog.js 的 openDialog，头上进度条两段）。平台照外壳问到的连接（挂着的几个平台），先选着 `preferred`（首页选中的连接的平台）。
// 填好的照平台的规矩查：QQ 号只有数字，别的平台不带空白、冒号、引号，48 个字以内（宿主的规矩，施工 I-79）；不对的提示一句、不关。
import { element, say } from '../../shell/i18n.js';
import { openDialog } from '../../shell/dialog.js';
import { mark, platform, venueKinds } from '../home/platforms.js';

/** 开浮窗：`platforms` 是挂着的几个平台，`bind(platform, id)` 存（交回存没存成）。 */
export function openBind(platforms, preferred, bind) {
  let chosen = platforms.includes(preferred) ? preferred : platforms[0] || '';
  const dialog = openDialog(say('bind/pick'));
  const pick = () => {
    dialog.setTitle(say('bind/pick'));
    dialog.setSteps(1, 2);
    const list = element('div', 'platforms');
    for (const id of platforms) {
      const shown = platform(id);
      const choice = element('button', `pf${chosen === id ? ' on' : ''}`);
      const tile = element('span', `pi pf-${id}`);
      tile.append(mark(shown));
      choice.append(tile, element('b', null, shown.name()));
      choice.addEventListener('click', () => {
        chosen = id;
        fill();
      });
      list.append(choice);
    }
    dialog.body.replaceChildren(list);
    const next = element('button', 'btn primary', say('wizard/next'));
    next.disabled = !chosen;
    next.addEventListener('click', fill);
    dialog.foot.replaceChildren(element('span', 'grow'), next);
  };
  const fill = () => {
    dialog.setTitle(say('bind/fill', { platform: platform(chosen).name() }));
    dialog.setSteps(2, 2);
    const word = venueKinds(chosen).private?.id() || say('venues/user-id');
    const input = element('input', 'input');
    input.placeholder = word;
    const digits = chosen === 'qq';
    if (digits) input.inputMode = 'numeric';
    const failed = element('p', 'failed');
    const box = element('div', 'fill');
    box.append(element('span', 'soft', word), input, element('span'));
    dialog.swap([box, failed]);
    const back = element('button', 'btn ghost', say('wizard/back'));
    back.addEventListener('click', pick);
    const done = element('button', 'btn primary', say('settings-page/bind'));
    done.disabled = true;
    input.addEventListener('input', () => {
      done.disabled = input.value.trim() === '';
      failed.textContent = '';
    });
    done.addEventListener('click', async () => {
      const typed = input.value.trim();
      if (!validId(chosen, typed)) {
        failed.textContent = say(digits ? 'settings-page/bind-bad' : 'settings-page/bind-bad-text');
        return;
      }
      done.disabled = true;
      if (await bind(chosen, typed)) dialog.close();
      else done.disabled = false;
    });
    dialog.foot.replaceChildren(back, element('span', 'grow'), done);
    input.focus();
  };
  pick();
}

/** 平台 `on` 上的号 `typed` 写得对不对（宿主的规矩，施工 I-79）：QQ 号只有数字，别的平台不带空白、冒号、引号，48 个字以内。新建连接的
 *  「你的平台号」那一步也照它查（施工 I-182）。 */
export function validId(on, typed) {
  return on === 'qq' ? /^\d+$/.test(typed) : /^[^\s:"]{1,48}$/.test(typed);
}
