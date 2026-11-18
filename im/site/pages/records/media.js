// 消息记录里带的东西现取现看（施工 C-1 中下补，设计 04 第一节第 1 条）：点占位，经页面服务的 records.media 去平台上取：图在浮层里
// 看，别的（文件、视频）交给浏览器下载；不落在 im 的盘上。取不到的说取不到了，可能已经失效。
import { element, say } from '../../shell/i18n.js';
import { closeButton, openLayer, toast } from '../../shell/dialog.js';

/** 拒的原因到提示的字。 */
const REFUSED = {
  gone: 'records/media-gone',
  not_connected: 'records/media-offline',
  not_fetchable: 'records/media-unfetchable',
  too_large: 'records/media-too-large',
};

/** 取场所 `venue` 里编号 `msg` 那一条带的编号 `id` 那一样（`name` 是记下的名字），`request` 调宿主的方法。 */
export async function openMedia(request, { venue, msg, id, name }) {
  const box = element('div', 'dialog records-viewer');
  const body = element('div', 'records-viewer-body', say('records/media-loading'));
  // 图看完、浮层怎么关的都放掉取来的那一份（施工 AUD-4，审计 #117：以前一张大图一份，一直不放）。
  let shownUrl = null;
  let closed = false;
  const close = openLayer(box, () => {
    closed = true;
    if (shownUrl) URL.revokeObjectURL(shownUrl);
    shownUrl = null;
  });
  box.append(element('div', 'records-viewer-head'), body);
  box.firstChild.append(element('span', 'grow', name || ''), closeButton(close));
  let got;
  try {
    got = await request('records.media', { venue, msg, id });
  } catch (reason) {
    close();
    toast(say(REFUSED[reason] || 'records/media-gone'));
    return;
  }
  const raw = atob(got.data || '');
  const bytes = new Uint8Array(raw.length);
  for (let i = 0; i < raw.length; i += 1) bytes[i] = raw.charCodeAt(i);
  const url = URL.createObjectURL(new Blob([bytes], { type: got.type || 'application/octet-stream' }));
  if (String(got.type || '').startsWith('image/')) {
    // 取回来以前浮层已经关了的：不画了，放掉。
    if (closed) {
      URL.revokeObjectURL(url);
      return;
    }
    shownUrl = url;
    const picture = element('img', 'records-viewer-image');
    picture.src = url;
    picture.alt = got.name || name || '';
    body.replaceChildren(picture);
    return;
  }
  // 别的交给浏览器下载，浮层收起。
  const link = element('a');
  link.href = url;
  link.download = got.name || name || `${msg}-${id}`;
  document.body.append(link);
  link.click();
  link.remove();
  close();
  setTimeout(() => URL.revokeObjectURL(url), 60000);
}
