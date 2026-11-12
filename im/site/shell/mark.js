// 名字后面的「默认 / 已改」小标签（设计 11 G3，施工 I-165）：会话浮窗的格、插件表单都用它，照网页软件「写过的露 ↺ 恢复默认」。
// 没写的「默认」，悬停说跟着谁；写了的「已改」，悬停换成「恢复默认」，点了调 `restore`。样子在 shell/mark.css。
import { element, say } from './i18n.js';

/** `changed` 是写了没有；没写的悬停写 `follows`（跟着谁、现在是什么）。 */
export function fieldMark(changed, follows, restore) {
  if (changed) {
    const button = element('button', 'field-mark changed');
    button.type = 'button';
    button.append(element('span', 'field-mark-was', say('field/changed')), element('span', 'field-mark-undo', say('field/restore')));
    button.addEventListener('click', restore);
    return button;
  }
  const tag = element('span', 'field-mark', say('field/default'));
  if (follows) tag.title = follows;
  return tag;
}
