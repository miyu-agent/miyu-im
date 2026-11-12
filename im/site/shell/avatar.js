// 头像（施工 I-7 上修）：先画占位（名字的第一个字、平台的标志、图标），页面服务转来的图到了再换上。取到过的地址记着，以后直接画图
// （页面服务让浏览器记十分钟，不再去取），页面重画时不再先闪一下占位。取不到的照旧是占位。
const loaded = new Set();

/** 在 `round`（已经画好占位的头像）里换上地址是 `url` 的图；换上时先调 `onShown`（去掉占位的底色这类）。 */
export function picture(round, url, onShown) {
  const shown = document.createElement('img');
  shown.alt = '';
  shown.src = url;
  const show = () => {
    onShown?.();
    round.replaceChildren(shown);
  };
  if (loaded.has(url)) {
    show();
    return;
  }
  shown.addEventListener('load', () => {
    loaded.add(url);
    show();
  });
}
