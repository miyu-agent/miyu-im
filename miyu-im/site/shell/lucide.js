// 插件声明里写的图标（施工 I-9 上）：Lucide 的名字，照整套图标（lucide-all.json，ISC 许可，同 WebUI 那一份）画。整套四百多 KiB，
// 头一次要画时才读，读到以前先画一个空的框，读到了填上；读不到、认不出的名字照外壳的拼图图标。

const SVG = 'http://www.w3.org/2000/svg';

/** 读整套图标：只读一次，读不到的是空表。 */
let loading = null;
function all() {
  loading ??= fetch(new URL('lucide-all.json', import.meta.url))
    .then((answer) => (answer.ok ? answer.json() : {}))
    .then((read) => read.icons || {})
    .catch(() => ({}));
  return loading;
}

/** Lucide 图标 `name`：24 的画布；`fallback` 是认不出时换上的外壳图标（一个 `svg` 元素）。 */
export function lucide(name, fallback) {
  const svg = document.createElementNS(SVG, 'svg');
  svg.setAttribute('viewBox', '0 0 24 24');
  svg.setAttribute('aria-hidden', 'true');
  svg.setAttribute('class', 'icon lucide');
  all().then((icons) => {
    const parts = typeof name === 'string' ? icons[name] : null;
    if (!parts) {
      if (fallback) svg.replaceWith(fallback);
      return;
    }
    for (const [tag, attrs] of parts) {
      const part = document.createElementNS(SVG, tag);
      for (const [key, value] of Object.entries(attrs)) part.setAttribute(key, value);
      svg.append(part);
    }
  });
  return svg;
}
