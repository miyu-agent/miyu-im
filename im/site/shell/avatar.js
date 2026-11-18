// 头像（施工 I-7 上修）：先画占位（名字的第一个字、平台的标志、图标），页面服务转来的图到了再换上。取到过的记着，以后直接画图
// （页面重画时不再先闪一下占位）。取不到的照旧是占位。
// 头像要登录才给（施工 AUD-4，审计 #113）：`<img>` 带不了会话号，照 fetch 带着 `Authorization: Bearer` 取，取回来的放进 blob 地址
// 再画。浏览器照页面服务的 `Cache-Control` 记十分钟，同一页里取到过的照记着的 blob 地址画，不再去取。
import { stored } from './login.js';

/** 地址 → 取回来的 blob 地址；取着的是那一次的 Promise（同一个头像同时画几处只取一次）。 */
const loaded = new Map();

/** 取地址是 `url` 的头像：交回 blob 地址，取不到的交回空的。 */
function fetched(url) {
  let got = loaded.get(url);
  if (got === undefined) {
    const session = stored();
    got = fetch(url, { headers: session ? { Authorization: `Bearer ${session}` } : {} })
      .then((response) => (response.ok ? response.blob() : null))
      .then((blob) => (blob ? URL.createObjectURL(blob) : null))
      .catch(() => null)
      .then((shown) => {
        // 取不到的不记着：下回（登录了、平台连上了）再取。
        if (shown) loaded.set(url, shown);
        else loaded.delete(url);
        return shown;
      });
    loaded.set(url, got);
  }
  return Promise.resolve(got);
}

/** 在 `round`（已经画好占位的头像）里换上地址是 `url` 的图；换上时先调 `onShown`（去掉占位的底色这类）。 */
export function picture(round, url, onShown) {
  const show = (blob) => {
    const shown = document.createElement('img');
    shown.alt = '';
    shown.src = blob;
    onShown?.();
    round.replaceChildren(shown);
  };
  const known = loaded.get(url);
  if (typeof known === 'string') {
    show(known);
    return;
  }
  fetched(url).then((blob) => {
    if (blob) show(blob);
  });
}
