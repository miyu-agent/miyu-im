// 填 token 数的格认 k、m 的写法（施工 I-202，项目主人 11-10：使用限制页填数值不支持「多少 k、多少 m」，不方便）。
//
// - `200k` 是 200,000，`1.5m` 是 1,500,000，大小写都认；也认照三位一撇、带空格、下划线写的（`2,000,000`）。
// - 算出来不是整数的（`1.2345k`、`0.0001k`）、不是数的交回 NaN，调的一方照写错办。

/** 把填的字 `text` 读成 token 数：整数，读不成的是 NaN。 */
export function tokensOf(text) {
  const typed = String(text).trim().replace(/[,_\s]/g, '');
  const read = /^(\d+(?:\.\d+)?)([kKmM]?)$/.exec(typed);
  if (!read) return NaN;
  const times = { '': 1, k: 1e3, m: 1e6 }[read[2].toLowerCase()];
  // 小数乘出来的浮点误差（1.1 × 1000）先抹掉再看是不是整数。
  const number = Math.round(Number(read[1]) * times * 1e6) / 1e6;
  return Number.isInteger(number) ? number : NaN;
}
