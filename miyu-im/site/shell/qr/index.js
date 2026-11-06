// 二维码（设计 11，施工 I-167：设置页「访问」打开局域网时，局域网网址画成二维码，手机扫了打开）。照网页软件的 src/lib/qr/（W-1086）
// 原样搬来：编码用拷来的 qrcode-generator 2.0.4（`vendor/qrcode.mjs`，MIT，原样没改；页面不往外连），字节模式、纠错 M 级、版本照
// 内容自动挑。这里只把字照 UTF-8 交给它（它自己的 `stringToBytes` 只取每个字的低 8 位）、交回一格格的明暗，再写成 SVG 的一条路径。纯的。

import qrcode from './vendor/qrcode.mjs';

/** 四边留白几格（规范要求至少 4 格，扫码才认得出边） */
export const QUIET = 4;

/**
 * 一段字编成二维码：`rows[y][x]` 是真的那一格是深色。
 * @param {string} text
 * @returns {boolean[][]}
 */
export function qrMatrix(text) {
  const code = qrcode(0, 'M');
  // 先照 UTF-8 拆成字节，一个字节一个字交进去（它照低 8 位取，取出来就是这些字节）
  code.addData(String.fromCharCode(...new TextEncoder().encode(text)), 'Byte');
  code.make();
  const n = code.getModuleCount();
  return Array.from({ length: n }, (_, y) => Array.from({ length: n }, (_, x) => code.isDark(y, x)));
}

/**
 * 深色的格子写成 SVG 路径（`M x y h1 v1 h-1 z` 一格一段，坐标已经加上四边留白）；配 `viewBox="0 0 <size> <size>"` 画。
 * @param {boolean[][]} rows
 * @returns {{path: string, size: number}}
 */
export function qrPath(rows) {
  const parts = [];
  rows.forEach((row, y) => row.forEach((dark, x) => {
    if (dark) parts.push(`M${x + QUIET} ${y + QUIET}h1v1h-1z`);
  }));
  return { path: parts.join(''), size: rows.length + QUIET * 2 };
}
