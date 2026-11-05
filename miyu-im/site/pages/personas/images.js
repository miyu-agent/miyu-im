// 人格的头像、背景图（设计 09，施工 I-146；照网页软件的 avatars.js、upload.js）：列表、persona.get 里只有版本，图经 persona.avatar、
// persona.background 取（base64），照「人格@版本」缓存成 blob 地址，版本变了重取。换：先在浏览器里缩小（2026-10-11 项目主人定：先缩
// 再传，核心的上限只当兜底），再用 blob.open、blob.write、blob.close 分块传，交回内容哈希给 persona.set。
import { core } from '../../shell/core.js';

/** 头像长边、背景图长边最多多少像素（同网页软件设置项 avatar_side、background_side 的出厂值）。 */
export const AVATAR_SIDE = 512;
export const BACKGROUND_SIDE = 2560;

/** 核心收头像、背景图的上限：缩小以后还超的再压。 */
export const AVATAR_BYTES = 1024 * 1024;
export const BACKGROUND_BYTES = 5 * 1024 * 1024;

/** 一块传多少字节（核心收的上限 512 KiB）、连着几次没往前走就不再接着传。 */
const CHUNK = 256 * 1024;
const TRIES = 3;

/** 核心收的几种图。 */
const KINDS = ['image/png', 'image/jpeg', 'image/webp'];

/** 「人格@版本」→ blob 地址；取到了叫听着的重画。 */
const urls = new Map();
const loading = new Set();
const listeners = new Set();

/** 读到一张新的图时叫 `listener`；交回不听了的。 */
export function onImage(listener) {
  listeners.add(listener);
  return () => listeners.delete(listener);
}

/** 人格 `id` 版本 `version` 的 `kind`（avatar、background）的地址：缓存里有的交回，没有的去取（取到了叫听着的），先交 null。 */
export function imageUrl(request, kind, id, version) {
  if (!version) return null;
  const key = `${kind}:${id}@${version}`;
  const got = urls.get(key);
  if (got) return got;
  if (!loading.has(key)) fetchImage(request, kind, id, key);
  return null;
}

async function fetchImage(request, kind, id, key) {
  loading.add(key);
  try {
    const got = await core(request, `persona.${kind}`, { persona: id });
    if (!got?.data) return;
    const bytes = Uint8Array.from(atob(got.data), (c) => c.charCodeAt(0));
    urls.set(`${kind}:${id}@${got[kind]}`, URL.createObjectURL(new Blob([bytes], { type: got.media_type })));
    for (const listener of listeners) listener();
  } catch {
    // 取不到的照没有画。
  } finally {
    loading.delete(key);
  }
}

/** 要传的那张图：核心收的类型、长边不超过 `side`、不比 `maxBytes` 大的照原样；别的画到长边 `side` 的画布上重新存，先 WebP，存不了
 *  的存 `fallback`，还大的存 JPEG（透明的地方铺白）。都超的交最小的那一份，由核心照上限拒。 */
export async function shrink(file, side, { fallback = 'image/png', maxBytes = Infinity } = {}) {
  const bitmap = await createImageBitmap(file);
  const scale = Math.min(1, side / Math.max(bitmap.width, bitmap.height));
  if (scale === 1 && KINDS.includes(file.type) && file.size <= maxBytes) {
    bitmap.close();
    return file;
  }
  const width = Math.max(1, Math.round(bitmap.width * scale));
  const height = Math.max(1, Math.round(bitmap.height * scale));
  const encode = (type, quality) => {
    const canvas = document.createElement('canvas');
    canvas.width = width;
    canvas.height = height;
    const paint = canvas.getContext('2d');
    if (!paint) return Promise.resolve(null);
    if (type === 'image/jpeg') {
      paint.fillStyle = '#ffffff';
      paint.fillRect(0, 0, width, height);
    }
    paint.drawImage(bitmap, 0, 0, width, height);
    return new Promise((resolve) => canvas.toBlob((blob) => resolve(blob?.type === type ? blob : null), type, quality));
  };
  const tried = [];
  for (const [type, quality] of [['image/webp', 0.9], [fallback, 0.9], ['image/jpeg', 0.8]]) {
    const got = await encode(type, quality);
    if (!got) continue;
    if (got.size <= maxBytes) {
      bitmap.close();
      return got;
    }
    tried.push(got);
  }
  bitmap.close();
  if (!tried.length) throw new Error('canvas');
  return tried.reduce((small, one) => (one.size < small.size ? one : small));
}

/** 把 `blob` 分块传给核心，交回内容哈希。`name` 起名用（avatar、background）。 */
export async function store(request, blob, name) {
  const call = (method, params) => core(request, method, params);
  const opened = await call('blob.open', {
    name: `${name}.${blob.type.split('/')[1] || 'png'}`,
    size: blob.size,
    ...(blob.type ? { media_type: blob.type } : {}),
  });
  let offset = 0;
  let stuck = 0;
  // offset 接不上（upload_offset）、存的时候没收齐（upload_incomplete）的照拒绝里的 received 接着传；连着几次没往前走的照原样抛。
  const resume = (error) => {
    const at = resumeAt(error);
    if (at === null) throw error;
    stuck = at > offset ? 0 : stuck + 1;
    if (stuck >= TRIES) throw error;
    offset = at;
  };
  for (;;) {
    while (offset < blob.size) {
      const bytes = new Uint8Array(await blob.slice(offset, offset + CHUNK).arrayBuffer());
      try {
        offset = (await call('blob.write', { upload: opened.upload, offset, data: toBase64(bytes) })).received;
        stuck = 0;
      } catch (error) {
        resume(error);
      }
    }
    try {
      return (await call('blob.close', { upload: opened.upload })).blob;
    } catch (error) {
      resume(error);
    }
  }
}

/** 照拒绝接着传的那两种交回从哪接着，别的交 null（纯逻辑）。 */
export function resumeAt(error) {
  const at = error?.data?.received;
  return (error?.reason === 'upload_offset' || error?.reason === 'upload_incomplete') && Number.isInteger(at) ? at : null;
}

/** 字节写成标准 base64（纯逻辑）。 */
export function toBase64(bytes) {
  let text = '';
  for (let at = 0; at < bytes.length; at += 0x8000) text += String.fromCharCode(...bytes.subarray(at, at + 0x8000));
  return btoa(text);
}
