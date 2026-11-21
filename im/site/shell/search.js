// 搜索框（施工 I-272，照网页包）：输入框左边一个放大镜。交回输入框本身（页面照旧读 .value、听 input），框外包的那一层是
// `searchBox.wrap`，放进页面的是它。
import { element, icon } from './i18n.js';

/** 带放大镜的搜索框：`placeholder` 是占位字，`className` 加在输入框上（同原来的 `input …`）。 */
export function searchBox(placeholder, className) {
  const wrap = element('label', 'search-box');
  const input = element('input', className ? `input ${className}` : 'input');
  input.type = 'search';
  input.placeholder = placeholder || '';
  input.autocomplete = 'off';
  wrap.append(icon('i-search'), input);
  input.wrap = wrap;
  return input;
}
