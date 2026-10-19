// 首页（设计 01 第八节第 2 条，施工 I-6 上）：一个连接都没有的写一句、正中留给新建连接（引导随 I-6 中）；有连接的「连接」一栏，
// 一个机器人一张卡：头像（暂时是平台的字母，取头像随 I-6 中）、平台的角标、号、平台和实现、在线和连了多久。今天的数字、运行日志
// 随 I-6 下。连了多久每分钟照本机的钟重画一次（不问宿主）。
import { element, say } from '../../shell/i18n.js';

/** 平台的样子：角标上的字、角标的颜色（class）。 */
const PLATFORMS = { qq: { badge: 'Q', name: () => say('platform/qq') } };

/** 在 `view` 里起首页，交回 update(connections)、dispose()。 */
export function mountHome(view) {
  let connections = [];
  const draw = () => {
    if (connections.length === 0) {
      view.replaceChildren(empty());
      return;
    }
    const head = element('div', 'view-head');
    head.append(element('h1', null, say('nav/home')));
    const section = element('div', 'section-head');
    section.append(
      element('h2', null, say('home/connections')),
      element('span', 'faint', say('home/count', { count: connections.length })),
    );
    const grid = element('div', 'conn-grid');
    for (const one of connections) grid.append(tile(one));
    view.replaceChildren(head, section, grid);
  };
  const timer = setInterval(draw, 60000);
  return {
    update(now) {
      connections = now;
      draw();
    },
    dispose() {
      clearInterval(timer);
    },
  };
}

function empty() {
  const box = element('div', 'empty');
  const inner = element('div');
  inner.append(element('h2', null, say('home/empty-title')), element('p', null, say('home/empty-text')));
  box.append(inner);
  return box;
}

function tile(one) {
  const platform = PLATFORMS[one.platform] || { badge: one.platform.slice(0, 1).toUpperCase(), name: () => one.platform };
  const card = element('div', 'conn-tile');
  const bot = element('span', 'bot');
  bot.append(element('span', 'avatar', platform.badge), element('span', `pf-badge pf-${one.platform}`, platform.badge));
  const grow = element('div', 'grow');
  const line = one.implementation
    ? say('conn/line', { platform: platform.name(), implementation: one.implementation, version: one.version })
    : platform.name();
  const state = element('div', 'faint');
  state.append(element('span', 'dot on'), document.createTextNode(` ${since(one)}`));
  grow.append(element('b', null, one.bot), element('div', 'soft', line), state);
  card.append(bot, grow);
  return card;
}

/** 「号 · 已连多久」。 */
function since(one) {
  const minutes = Math.floor((Date.now() - one.since) / 60000);
  if (minutes < 1) return say('conn/since-now', { bot: one.bot });
  if (minutes < 60) return say('conn/since-minutes', { bot: one.bot, n: minutes });
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return say('conn/since-hours', { bot: one.bot, n: hours });
  return say('conn/since-days', { bot: one.bot, n: Math.floor(hours / 24) });
}
