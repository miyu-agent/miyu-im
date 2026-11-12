// 模型页的纯逻辑（设计 09，施工 I-149；照网页软件 settings/model.js、model-bits.js 里模型、供应商那几样）：配置的键怎么拼、
// config.set 的 expect、存一家供应商要发的几项、删一家要删的几项、模型的显示名和那一行暗字、搜索。都不碰页面，测试照它测。

/** 模型页写哪一层：只有系统那一层（单用户）。 */
export const LAYER = 'system';

/** 键里的一段：只有字母、数字、-、_ 的照写，别的加引号。 */
export const keySegment = (name) =>
  /^[A-Za-z0-9_-]+$/.test(name) ? name : `"${name.replaceAll('\\', '\\\\').replaceAll('"', '\\"')}"`;

/** 带占位的键换成真键：`providers.<id>.models.<model>.window` + {id, model} → `providers.dev.models."a/b".window`。 */
export function realKey(template, names) {
  return template
    .split('.')
    .map((part) => (part.startsWith('<') ? keySegment(names[part.slice(1, -1)] ?? part) : part))
    .join('.');
}

/** config.get 读到的一项在 `layer` 这一层写着什么：config.set 的 expect，写了的 {value}，没写的 {}。 */
export function expectFor(entry, layer = LAYER) {
  const had = entry?.layers?.find((one) => one.origin?.layer === layer);
  return had ? { value: had.value } : {};
}

/** 这一层写了这一项没有：写了的能「恢复默认」、删供应商时要删。 */
export const writtenIn = (entry, layer = LAYER) => Boolean(entry?.layers?.some((one) => one.origin?.layer === layer));

/** 值是环境变量引用（{env: X}）的交回名字。 */
export const envRef = (value) =>
  value && typeof value === 'object' && !Array.isArray(value) && typeof value.env === 'string' ? value.env : null;

/** 配置里的值写成一格字。 */
export function inputText(value) {
  if (value == null) return '';
  if (typeof value === 'string') return value;
  if (typeof value === 'number' || typeof value === 'boolean') return String(value);
  return JSON.stringify(value);
}

/** 模板 `template` 照 `names` 换成真键，配上 schema 里的那一项和 config.get 读到的；schema 里没有的是 null。 */
export function itemFor(schema, got, template, names, fact) {
  const spec = schema.items.find((one) => one.key === template);
  if (!spec) return null;
  const key = realKey(template, names);
  return { ...spec, key, entry: got.items?.[key] ?? null, ...(fact ? { fact } : {}) };
}

/** 一家供应商的 ID 的写法。 */
export const validId = (id) => /^[a-z][a-z0-9_-]{0,31}$/.test(id);

/** 粘贴的密钥存成的名字：`<供应商>-<时刻的 36 进制>`，不覆盖原来的。 */
export const secretName = (id, now) => `${id}-${now.toString(36)}`.slice(0, 64);

/** 写给人看的供应商名字：配置、目录给的，没有的写 ID。 */
export const providerName = (provider) =>
  (typeof provider.name === 'object' ? provider.name?.value : provider.name) || provider.id;

/** 名字重了的那几个。 */
export function duplicates(names) {
  const seen = new Set();
  const twice = new Set();
  for (const name of names) (seen.has(name) ? twice : seen).add(name);
  return twice;
}

/** 地址的主机名，不是网址的是 null。 */
export function hostOf(url) {
  if (typeof url !== 'string') return null;
  try {
    return new URL(url).host || null;
  } catch {
    return null;
  }
}

/** 地址从哪来：主机名，环境变量引用的写 $名字。 */
export const whereOf = (url) => hostOf(url) ?? (envRef(url) ? `$${envRef(url)}` : null);

/** 和别家同名的那几家的 ID（左边多写一行地址好分开）。 */
export function sameNamed(providers) {
  const names = duplicates(providers.map(providerName));
  return new Set(providers.filter((one) => names.has(providerName(one))).map((one) => one.id));
}

/** 存一家供应商的表要发的 changes：显示名、地址、接口、密钥，每项带 expect；空着的显示名、地址是删掉（这一层没写的不发），地址 null
 *  是不改（原来是环境变量、框空着）；密钥 keep 是不改。 */
export function providerChanges(id, form, got, secret) {
  const key = (field) => `providers.${keySegment(id)}.${field}`;
  const changes = [];
  const put = (field, change) => {
    const entry = got.items?.[key(field)] ?? null;
    if ('unset' in change && !writtenIn(entry)) return;
    changes.push({ key: key(field), ...change, expect: expectFor(entry) });
  };
  put('name', form.name.trim() ? { value: form.name.trim() } : { unset: true });
  if (form.base_url !== null) put('base_url', form.base_url.trim() ? { input: form.base_url.trim() } : { unset: true });
  if (form.driver) put('driver', { value: form.driver });
  if (form.key.kind === 'secret' && secret) put('key', { value: { secret } });
  if (form.key.kind === 'env' && form.key.value.trim()) put('key', { value: { env: form.key.value.trim() } });
  return changes;
}

/** 只换一家的密钥引用。 */
export function keyChange(id, ref, got) {
  const key = `providers.${keySegment(id)}.key`;
  return [{ key, value: ref, expect: expectFor(got.items?.[key] ?? null) }];
}

/** 删一家：这一层写了的 providers.<id>.* 各 unset。 */
export function providerRemoval(id, got) {
  const prefix = `providers.${keySegment(id)}.`;
  return Object.keys(got.items ?? {})
    .filter((key) => key.startsWith(prefix) && writtenIn(got.items[key]))
    .map((key) => ({ key, unset: true, expect: expectFor(got.items[key]) }));
}

/** 模型的显示名：目录给的名字，没有的写模型名。 */
export const modelName = (model) => model.facts?.name?.value ?? model.model;

/** 看得了图。 */
export const seesImages = (model) => (model.facts?.inputs?.value ?? []).includes('image');

/** 数写短：1000000 → 1M，128000 → 128k。 */
export function shortCount(count) {
  if (typeof count !== 'number') return '';
  if (count >= 1e6) return `${+(count / 1e6).toFixed(count % 1e6 ? 1 : 0)}M`;
  if (count >= 1e3) return `${+(count / 1e3).toFixed(count % 1e3 ? 1 : 0)}k`;
  return String(count);
}

/** 这一家每个模型都一样的「用不了」（没有密钥这类）：交回它，写一次；冷却中的不算。 */
export function sharedState(models) {
  const states = new Set(models.map((model) => model.state ?? 'ok'));
  const [only] = states;
  return states.size === 1 && only !== 'ok' && only !== 'cooling' ? only : null;
}

/** 照显示名、模型名、供应商名筛，空格分开的几个词都要对上。 */
export function filterModels(rows, query) {
  const words = query.toLowerCase().split(/\s+/).filter(Boolean);
  if (!words.length) return rows;
  return rows.filter((row) => {
    const hay = `${row.name}\n${row.model}\n${row.provider ?? ''}`.toLowerCase();
    return words.every((word) => hay.includes(word));
  });
}

/** 供应商的图标：核心给的 {svg, tint}，写成 data 地址；没有的 null。 */
export function logoPlan(logo) {
  if (!logo || typeof logo.svg !== 'string' || !logo.svg) return null;
  return { tint: Boolean(logo.tint), url: `data:image/svg+xml;charset=utf-8,${encodeURIComponent(logo.svg)}` };
}
