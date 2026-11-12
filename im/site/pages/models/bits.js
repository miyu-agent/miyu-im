// 模型页几块共用的小部件（设计 09，施工 I-149；照网页软件 model-bits.js、lib/logo.js）：供应商的图标、模型那一行暗字、测试连接的
// 结果、「正在读模型列表」、就地筛的搜索框。
import { element, say } from '../../shell/i18n.js';
import { filterModels, logoPlan, seesImages, shortCount } from './config.js';

/** 供应商的图标（2026-10-11 项目主人：有彩色的用彩色，其余单色照字色画）：核心给的 {svg, tint}；单色的用 CSS 遮罩铺字色，彩色的
 *  照原样画，都不把 SVG 插进页面；没有图的写名字的第一个字。 */
export function logoElement(logo, name) {
  const plan = logoPlan(logo);
  if (!plan) return element('span', 'model-logo initial', ([...String(name ?? '').trim()][0] ?? '').toUpperCase());
  if (!plan.tint) {
    const shown = element('img', 'model-logo');
    shown.src = plan.url;
    shown.alt = '';
    return shown;
  }
  const shown = element('span', 'model-logo tint');
  shown.style.setProperty('--logo', `url("${plan.url}")`);
  return shown;
}

/** 有没有图标这一格（老核心没有，不画图标位）。 */
export const hasLogos = (items) => (items ?? []).some((one) => one && typeof one === 'object' && 'logo' in one);

/** 一行暗字：窗口（`long` 的写「上下文」）、看得了图的写「看图」。 */
export function metaOf(model, long = false) {
  const count = shortCount(model.facts?.window?.value);
  const parts = [
    count ? (long ? say('models/meta-window', { n: count }) : count) : null,
    seesImages(model) ? say('models/meta-vision') : null,
  ];
  return parts.filter(Boolean).join(' · ');
}

/** 测试连接的结果那一行：只列了模型的写列出几个，发了一句的写模型和首字；没成的照原话，要模型名的接一句怎么再测。 */
export function testLine(result) {
  if (result.ok) {
    return result.checked === 'list'
      ? say('models/test-listed', { count: (result.models ?? []).length })
      : say('models/test-ok', { model: result.model, ms: result.first_token_ms });
  }
  const raw = result.error?.message ?? result.stage ?? '';
  return result.need_model ? `${raw} · ${say('models/test-need-model')}` : raw;
}

/** 「正在读模型列表」那一行。 */
export const loadingRow = () => element('p', 'faint', say('models/loading'));

/** 搜索框：照显示名、模型名、供应商名就地藏掉对不上的行（不重画，打字不丢焦点）；字记在 `memory[slot]`，重画以后照旧。 */
export function searchable(memory, slot, rows) {
  const input = element('input', 'input model-search');
  input.type = 'search';
  input.placeholder = say('models/search');
  input.value = memory[slot] ?? '';
  const empty = element('p', 'faint', say('models/no-match'));
  const apply = () => {
    memory[slot] = input.value;
    const shown = new Set(filterModels(rows, input.value));
    for (const row of rows) row.node.hidden = !shown.has(row);
    empty.hidden = shown.size > 0;
  };
  input.addEventListener('input', apply);
  apply();
  return { input, empty };
}
