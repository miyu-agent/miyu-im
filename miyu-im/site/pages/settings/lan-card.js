// 别的设备怎么打开（设计 11，施工 I-167）：设置页「访问」开着局域网时的一张卡，照网页软件的 lan-card.js（W-1086）：左边局域网网址的
// 二维码，右边「在别的设备上打开」、等宽的网址和「复制」，下面一句扫码怎么登录。二维码白底黑块、四边留白，深色主题也不反过来
// （反色的二维码有的手机扫不出来）；一格画成整像素，不糊边。样子在 settings.css 的 .lan-*。
import { element, say } from '../../shell/i18n.js';
import { copy } from '../../shell/dialog.js';
import { qrMatrix, qrPath } from '../../shell/qr/index.js';

const SVG = 'http://www.w3.org/2000/svg';

/** 一张卡：`url` 是局域网里打开这个页面的网址。 */
export function lanCard(url) {
  const card = element('div', 'lan-card');
  const side = element('div', 'lan-side');
  const address = element('div', 'lan-address');
  const button = element('button', 'btn', say('dialog/copy'));
  button.type = 'button';
  button.addEventListener('click', () => copy(url));
  address.append(element('code', 'lan-url', url), button);
  side.append(element('p', 'lan-label', say('settings-page/lan-open')), address, element('p', 'lan-scan', say('settings-page/lan-scan')));
  card.append(qrImage(url), side);
  return card;
}

/** 网址画成二维码（shell/qr/）。 */
function qrImage(url) {
  const { path, size } = qrPath(qrMatrix(url));
  const svg = document.createElementNS(SVG, 'svg');
  svg.setAttribute('class', 'lan-qr');
  svg.setAttribute('viewBox', `0 0 ${size} ${size}`);
  svg.setAttribute('shape-rendering', 'crispEdges');
  svg.setAttribute('role', 'img');
  svg.setAttribute('aria-label', url);
  const paper = document.createElementNS(SVG, 'rect');
  paper.setAttribute('width', String(size));
  paper.setAttribute('height', String(size));
  paper.setAttribute('fill', '#fff');
  const ink = document.createElementNS(SVG, 'path');
  ink.setAttribute('d', path);
  ink.setAttribute('fill', '#000');
  svg.append(paper, ink);
  return svg;
}
