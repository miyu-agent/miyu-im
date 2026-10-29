// 密钥输入框（施工 I-47，项目主人 10-27 验收：反向连接的 Token 默认没遮住，要眼睛图标看原文）：默认圆点遮着，右边一个眼睛按钮，
// 点一下露出原文、再点遮回去。新建连接、连接设置、插件设置里的密钥都用它。
import { element, icon, say } from './i18n.js';

/** 一个密钥框：交回 { box（放进页面的那一格）, input }。`className` 给输入框。 */
export function secretInput(className) {
  const box = element('span', 'secret');
  const input = element('input', className);
  input.type = 'password';
  input.autocomplete = 'off';
  input.spellcheck = false;
  const eye = element('button', 'icon-button secret-eye');
  eye.type = 'button';
  const show = (shown) => {
    input.type = shown ? 'text' : 'password';
    eye.title = say(shown ? 'secret/hide' : 'secret/show');
    eye.replaceChildren(icon(shown ? 'i-eye-off' : 'i-eye'));
  };
  eye.addEventListener('click', () => {
    show(input.type === 'password');
    input.focus();
  });
  show(false);
  box.append(input, eye);
  return { box, input };
}
