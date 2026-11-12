// 接一家供应商的纯逻辑（设计 09，施工 I-151；照网页软件 settings/onboard/logic.js 的设置页那一种，写的东西和 `miyu setup` 一样）：选一家
// 那一屏有哪几行、标什么；配置里的 ID 怎么起（目录里的、自定义的）；provider.test 带什么；存的时候写哪几项。设置页只写这一家，不写
// models.chat、不建池（那是第一次引导的事）。
import { providerName, validId } from './config.js';

/** 通用的二级域名：api.example.co.uk 取 example。 */
const SECOND_LEVEL = new Set(['co', 'com', 'net', 'org', 'edu', 'gov', 'ac']);

/** 目录里的编号写进配置用的 ID：别的字换成 -，不是字母开头的前面加 p-（302ai → p-302ai），最长 32。 */
export function configId(catalog) {
  let id = catalog.toLowerCase().replace(/[^a-z0-9_-]/g, '-');
  if (!/^[a-z]/.test(id)) id = `p-${id}`;
  return id.slice(0, 32);
}

/** 自定义的那一家的 ID：照地址的主机名起，本机的（localhost、IP）是 local，别的取倒数第二段；配置里有了的往后加 -2、-3。 */
export function customId(url, taken) {
  const host = new URL(url).hostname.replace(/^\[|\]$/g, '');
  let base;
  if (host === 'localhost' || /^[\d.]+$/.test(host) || host.includes(':')) {
    base = 'local';
  } else {
    const parts = host.split('.');
    let at = parts.length - 2;
    if (at > 0 && SECOND_LEVEL.has(parts[at])) at -= 1;
    base = parts[Math.max(0, at)];
  }
  const root = configId(base).slice(0, 29);
  let id = root;
  for (let n = 2; taken.has(id); n += 1) id = `${root}-${n}`;
  return id;
}

/** 人起的 ID 哪里不对：写法不对的 bad-id，重了的 taken-id，对的 null。 */
export function customIdProblem(id, taken) {
  if (!validId(id)) return 'bad-id';
  return taken.has(id) ? 'taken-id' : null;
}

/** 地址写得对不对：http://、https:// 开头、读得出主机名。 */
export function validUrl(text) {
  if (!/^https?:\/\//i.test(text)) return false;
  try {
    return Boolean(new URL(text).hostname);
  } catch {
    return false;
  }
}

/** 地址去掉前后空白和末尾的 /。 */
export const cleanUrl = (text) => text.trim().replace(/\/+$/, '');

/** 配好了的几家：密钥取得到的、没写密钥的（本机的服务这类）。写了密钥、引用的密钥没了的不算。 */
export const usableIds = (providers) => new Set(providers.filter((one) => !one?.key || one.key.set).map((one) => one.id));

/** 选一家那一屏的几行：常用的照目录 featured 的先后，本机的照 provider.detect 的先后。每行 {kind, id, name, configured, ...}；核心给图标
 *  的每一行带 logo。 */
export function providerRows(featured, detect, configured) {
  const keys = detect?.keys ?? [];
  const logos = featured.some((one) => one && typeof one === 'object' && 'logo' in one);
  const logoOf = (one) => (logos ? { logo: one?.logo ?? null } : {});
  const common = featured.map((one) => {
    const id = configId(one.id);
    const key = keys.find((found) => found.provider === one.id && found.supported);
    const as = configured.has(id) ? id : (keys.find((found) => found.provider === one.id && found.configured)?.configured ?? null);
    return { kind: 'catalog', id: one.id, name: one.name || one.id, supported: Boolean(one.supported), configured: as, env: key?.env ?? null, ...logoOf(one) };
  });
  const local = (detect?.local ?? []).map((one) => {
    let host = one.base_url;
    try {
      host = new URL(one.base_url).host;
    } catch {
      // 读不出的照原样写。
    }
    return { kind: 'local', id: one.provider, name: one.name || one.provider, base_url: one.base_url, host, configured: one.configured ?? null, ...logoOf(one) };
  });
  return { common, local };
}

/** 配置里已经有、常用和本机里都没有的几家（自定义的、搜出来接的），排在最前面（2026-10-23 项目主人）。 */
export function mineRows(providers, shown) {
  const taken = new Set(shown.map((row) => row.configured).filter(Boolean));
  return (providers ?? [])
    .filter((one) => one?.id && !taken.has(one.id))
    .map((one) => ({ kind: 'mine', id: one.id, name: providerName(one), configured: one.id, ...('logo' in one ? { logo: one.logo ?? null } : {}) }));
}

/** provider.test 的参数：配好的带 provider（重新设置贴了新密钥的照接口、地址当候选），别的带 candidate；要模型名的带 model。 */
export function testParams(target, key, model) {
  const params = {};
  if (target.kind === 'configured' && !(key && target.driver && target.base_url)) {
    params.provider = target.id;
  } else if (target.kind === 'configured') {
    params.candidate = { driver: target.driver, base_url: target.base_url, key };
  } else {
    const candidate = target.kind === 'custom' ? { driver: target.driver, base_url: target.base_url } : { catalog: target.catalog };
    if (key) candidate.key = key;
    params.candidate = candidate;
  }
  if (model) params.model = model;
  return params;
}

/** 存的时候写哪几项：没配过的这一家的密钥（本机的服务写 local = true）、目录编号改过写法的另写 catalog、自定义的写 driver、name、
 *  base_url；配好了的重新设置只写新的 key。交回 {id, changes}。 */
export function setupChanges(target, key, taken) {
  const changes = [];
  let id;
  if (target.kind === 'configured') {
    id = target.id;
    if (key) changes.push({ key: `providers.${id}.key`, value: key });
    return { id, changes };
  }
  id = target.kind === 'custom' ? target.id || customId(target.base_url, taken) : configId(target.catalog);
  const put = (field, value) => changes.push({ key: `providers.${id}.${field}`, value });
  if (target.kind !== 'custom' && id !== target.catalog) put('catalog', target.catalog);
  if (target.kind === 'custom') {
    put('driver', target.driver);
    if (target.name) put('name', target.name);
    changes.push({ key: `providers.${id}.base_url`, input: target.base_url });
  }
  if (target.kind === 'local') put('local', true);
  else if (key) put('key', key);
  return { id, changes };
}

/** 没测成的那一行写什么：要模型名的写「未获取到模型列表」（多一行填模型名），stage 是 config 的写「配置错误」，别的照出错的分类；有
 *  状态码的接 · 401。交回字的键、状态码、要不要填模型名、原话。 */
export function failureOf(result) {
  const error = result?.error ?? {};
  const message = typeof error.message === 'string' ? error.message : '';
  const status = typeof error.status === 'number' ? error.status : null;
  if (result?.need_model) return { key: 'add/no-list', status: null, needsModel: true, message };
  if (result?.stage === 'config') return { key: 'add/config-error', status: null, needsModel: false, message };
  const known = ['retryable', 'rate_limited', 'context_too_long', 'auth', 'quota', 'no_model', 'cooling', 'content_policy', 'bad_stream', 'empty_reply'];
  const kind = known.includes(error.class) ? error.class : 'other';
  return { key: `add/class-${kind.replaceAll('_', '-')}`, status, needsModel: false, message };
}
