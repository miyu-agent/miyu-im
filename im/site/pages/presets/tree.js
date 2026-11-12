// 功能树（设计 09，施工 I-147；照网页软件的 feature-tree.js，核心设计 52「功能分组」第四节）：照 preset.get、preset.set 回应的
// sections 画，组怎么分、先后、名字核心都算好了，这里不翻译、不排序、不认编号。
//
// - 一组：头一行组名、暗字「开了几项」；package 的多一个总开关（拨一下这几个功能全开、全关），别的只当标题（2026-10-27 项目主人：组
//   一关就是二十个，误点代价大）。技能、脚本、MCP 三组空的写「还没有…」。
// - 组下面直接是功能，带了几件工具的工具缩进一格，功能关着时工具不能拨；两列，照行数摊匀、先后不变。
// - 技能、脚本、MCP 三组的功能：by 是 global 的写「跟随全局」；by 是 preset、预设 unlisted 是 on 的给「跟随全局默认」（2026-10-20
//   项目主人）。
import { element, say } from '../../shell/i18n.js';

/** 空的时候写「还没有…」的三组。 */
const EMPTY_KINDS = ['skills', 'scripts', 'mcp'];

/** 一组占几行（纯逻辑）。 */
export const rowsOf = (section) =>
  1 + (section.items.length || 1) + section.items.reduce((count, item) => count + (item.tools?.length ?? 0), 0);

/** 两列：一组不拆开，找让高的那一列最矮的分点（纯逻辑）。 */
export function split(sections) {
  const rows = sections.map(rowsOf);
  const total = rows.reduce((sum, count) => sum + count, 0);
  let best = 0;
  let tallest = Infinity;
  let left = 0;
  for (let at = 0; at <= rows.length; at++) {
    const tall = Math.max(left, total - left);
    if (tall < tallest) {
      tallest = tall;
      best = at;
    }
    left += rows[at] ?? 0;
  }
  return [sections.slice(0, best), sections.slice(best)];
}

/** 照组名、功能名、工具名筛（不分大小写）：组名对上的整组留着，功能对上的留它，只有工具对上的留那几件和它的功能（纯逻辑）。 */
export function filterTree(sections, query) {
  const wanted = query.trim().toLowerCase();
  if (!wanted) return sections;
  const hit = (text) => Boolean(text) && text.toLowerCase().includes(wanted);
  return sections.flatMap((section) => {
    if (hit(section.name)) return [section];
    const items = section.items.flatMap((item) => {
      if (hit(item.name) || hit(item.id)) return [item];
      const tools = (item.tools ?? []).filter((tool) => hit(tool.name) || hit(tool.id));
      return tools.length ? [{ ...item, tools }] : [];
    });
    return items.length ? [{ ...section, items }] : [];
  });
}

/** 旧核心不给 sections 的：照 features 拼一组，没装的不列（纯逻辑）。 */
export function fallbackSections(features, name) {
  const items = features.filter((item) => item.installed !== false).map((item) => ({ ...item, kind: 'feature' }));
  return [{ id: 'features', kind: 'group', name, on: items.filter((item) => item.on).length, total: items.length, items }];
}

/** 画功能树：开关照 `set(changes)` 写（交回 Promise<bool>，存不成的拨回去）；`unlisted` 是预设的那一格。 */
export function featureTree(sections, set, unlisted) {
  const box = element('div', 'preset-tree');
  for (const column of split(sections).filter((one) => one.length)) {
    const shown = element('div', 'preset-tree-col');
    shown.append(...column.map((section) => drawSection(section, set, unlisted)));
    box.append(shown);
  }
  return box;
}

function drawSection(section, set, unlisted) {
  const items = section.items;
  const block = element('section', 'preset-block');
  const head = element('div', 'preset-row head');
  const count = `${section.on ?? items.filter((item) => item.on).length}/${section.total ?? items.length}`;
  head.append(element('span', 'preset-name', section.name ?? section.id), element('span', 'preset-count', count));
  if (section.kind === 'package' && items.length) {
    head.append(
      toggle(items.some((item) => item.on), (next) => {
        const changes = items.filter((item) => Boolean(item.on) !== next).map((item) => ({ key: `features.${item.id}`, value: next }));
        return changes.length ? set(changes) : Promise.resolve(true);
      }),
    );
  }
  block.append(head);
  if (!items.length && EMPTY_KINDS.includes(section.kind)) {
    block.append(element('p', 'preset-empty', say(`presets/empty-${section.kind}`)));
  }
  for (const item of items) block.append(...drawItem(item, section.kind, set, unlisted));
  return block;
}

function drawItem(item, kind, set, unlisted) {
  const row = element('div', 'preset-row');
  const name = element('span', 'preset-name', item.name ?? item.id);
  if (item.summary) name.title = item.summary;
  row.append(name);
  if (EMPTY_KINDS.includes(kind)) row.append(...marks(item, set, unlisted));
  row.append(toggle(Boolean(item.on), (on) => set([{ key: `features.${item.id}`, value: on }])));
  const tools = item.tools ?? [];
  if (!tools.length) return [row];
  const box = element('div', item.on ? 'preset-tools' : 'preset-tools off');
  for (const tool of tools) {
    const line = element('div', 'preset-row tool');
    const one = toggle(Boolean(tool.on), (on) => set([{ key: `tools.${tool.id}`, value: on }]));
    if (!item.on) one.disabled = true;
    line.append(element('span', 'preset-name', tool.name ?? tool.id), one);
    box.append(line);
  }
  return [row, box];
}

/** 谁定的：跟随全局的写一句，预设写了的给「跟随全局默认」（删掉预设里写的那一项）。 */
function marks(item, set, unlisted) {
  if (item.by === 'global') return [element('span', 'preset-by', say('presets/by-global'))];
  if (item.by !== 'preset' || unlisted !== 'on') return [];
  const back = element('button', 'btn ghost preset-back', say('presets/follow-global'));
  back.addEventListener('click', () => set([{ key: `features.${item.id}`, unset: true }]));
  return [back];
}

/** 一个开关：拨了先变样子，`change(on)` 交回假的（存不成）拨回去。 */
function toggle(on, change) {
  const button = element('button', on ? 'switch on' : 'switch');
  button.append(element('i'));
  let now = on;
  button.addEventListener('click', async () => {
    now = !now;
    button.classList.toggle('on', now);
    if (!(await change(now))) {
      now = !now;
      button.classList.toggle('on', now);
    }
  });
  return button;
}
