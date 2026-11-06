// 一组设置一张卡片（设计 11 G1，施工 I-165）：会话浮窗、插件面板、设置页、使用限制页共用这一份，照网页软件设置页的 groupBlock。
// 组名写在卡片第一行，卡和卡之间留空；样子在 shell/sect.css，尺寸是 base.css 的变量（--sect-*）。
import { element } from './i18n.js';

/** 一张卡片：`title` 是组名（照语言挑好的字，空的不写组名），`hint` 是组名下面一句小字，`aside` 是组名那一行右边的控件（切换、
 *  时间段，都可以不写）。交回 { box, body }，几行放进 body。 */
export function sect(title, hint, aside) {
  const box = element('section', 'sect');
  if (aside) {
    const head = element('div', 'sect-head');
    head.append(element('div', 'sect-name', title || ''), aside);
    box.append(head);
  } else if (title) box.append(element('div', 'sect-name', title));
  if (hint) box.append(element('p', 'sect-hint', hint));
  const body = element('div', 'sect-body');
  box.append(body);
  return { box, body };
}

/** 照 `sections`（sect 交回的那几张）留下有内容的卡片：藏得一行不剩的不画。 */
export function filled(sections) {
  return sections.filter((one) => one.body.childElementCount > 0).map((one) => one.box);
}
