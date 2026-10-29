// 首页（设计 01 第八节第 2 条，施工 I-6 上起）：一个连接都没有的正中一个「新建连接」；有连接的「连接」一栏，一个机器人一张卡：
// 头像（页面服务转的，取不到的写平台的字母）、平台的角标、名字（没问到的写号）、平台和实现、号和连了多久；断开了的卡留着，写断开
// 多久、在重连（施工 I-6 中）。齿轮打开连接设置，「新建连接」打开引导。下面是统计（I-85 下从统计页并进来，stats.js）、插件的首页
// 卡片（施工 I-8 下，plugin-cards.js）和最近的运行日志（只读，能按级别筛），宿主推了变了跟着连接一起再问（施工 I-6 下）。连了多久
// 每分钟照本机的钟重画一次（不问宿主）。
// 连接不止一个时连接那一排就是切换器（施工 I-85 中，设计 07 第二节第 2 条）：前面一张「全部」，点一张选中它（shell/connection.js），
// 选中的底色深一档；齿轮照旧开那个连接的设置（问、存都带它的平台）。统计、日志跟着选中的连接问（施工 I-85 下）。
import { element, icon, say } from '../../shell/i18n.js';
import { picture } from '../../shell/avatar.js';
import { mark, platform } from './platforms.js';
import { openSettings } from './settings.js';
import { pluginCards } from './plugin-cards.js';
import { activityRows, namesOf } from './activity.js';
import { statsPanel } from './stats.js';
import { chooseConnection, chosenOf, EVERY, onConnection } from '../../shell/connection.js';
import { openWizard } from './wizard.js';

/** 在 `view` 里起首页：`request` 调宿主的方法。交回 update(connections)、dispose()。 */
export function mountHome(view, { request }) {
  let connections = [];
  let wizard = null;
  let logs = [];
  let level = 'all';
  let names = new Map();
  // 日志看活动还是原始的（施工 I-19）：记在这台浏览器上。
  let mode = 'activity';
  try {
    if (localStorage.getItem('miyu-im-log-mode') === 'raw') mode = 'raw';
  } catch {
    // 记不住就算了。
  }
  const plugins = pluginCards(request);
  const stats = statsPanel(request);
  const refresh = async () => {
    const picked = chosenOf(connections);
    try {
      const wanted = mode === 'activity' ? { level: 'all', limit: 120 } : { level, limit: 30 };
      if (picked) wanted.connection = picked.id;
      let cards;
      [logs, cards] = await Promise.all([
        request('logs', wanted).then((answer) => answer.logs),
        request('cards.get').catch(() => null),
      ]);
      names = namesOf(cards);
    } catch {
      return;
    }
    stats.update(connections, picked, names);
    draw();
  };
  const newButton = (className) => {
    const button = element('button', className);
    button.append(icon('i-plus'), document.createTextNode(say('home/new')));
    button.addEventListener('click', () => {
      wizard = openWizard(request);
      wizard.update(connections);
    });
    return button;
  };
  const draw = () => {
    if (connections.length === 0) {
      const box = element('div', 'empty');
      const inner = element('div');
      inner.append(
        element('h2', null, say('home/empty-title')),
        element('p', null, say('home/empty-text')),
        newButton('btn primary lg'),
      );
      box.append(inner);
      view.replaceChildren(box);
      return;
    }
    const head = element('div', 'view-head');
    head.append(element('h1', null, say('nav/home')));
    const section = element('div', 'section-head');
    section.append(
      element('h2', null, say('home/connections')),
      element('span', 'faint', say('home/count', { count: connections.length })),
      element('span', 'grow'),
      newButton('btn'),
    );
    const grid = element('div', 'conn-grid');
    const picking = connections.length > 1;
    const picked = chosenOf(connections);
    if (picking) grid.append(everyTile(connections, picked === null));
    for (const one of connections) {
      grid.append(tile(one, request, picking ? { on: picked?.id === one.id } : null));
    }
    const choose = (chosen) => {
      if (chosen === 'activity' || chosen === 'raw') {
        mode = chosen;
        try {
          localStorage.setItem('miyu-im-log-mode', chosen);
        } catch {
          // 记不住就算了。
        }
      } else {
        level = chosen;
      }
      refresh();
    };
    const tagged = !picked && picking;
    view.replaceChildren(head, section, grid, stats.head, stats.body, plugins.box, logPanel(logs, { level, mode, names, tagged }, choose));
  };
  const timer = setInterval(draw, 60000);
  const unlisten = onConnection(() => {
    draw();
    refresh();
  });
  draw();
  return {
    update(now) {
      connections = now;
      wizard?.update(now);
      draw();
      refresh();
    },
    dispose() {
      clearInterval(timer);
      unlisten();
      stats.dispose();
    },
  };
}

/** 最近的运行日志：头上切「活动」「原始日志」（施工 I-19）；原始的再按级别筛（全部、提醒、出错），一行一条，新的在前。 */
function logPanel(logs, { level, mode, names, tagged }, choose) {
  const panel = element('div', 'panel logs');
  const head = element('h3', null, say('home/logs'));
  const modes = element('span', 'log-filter');
  for (const [value, label] of [['activity', say('logs/activity')], ['raw', say('logs/raw')]]) {
    const button = element('button', value === mode ? 'on' : null, label);
    button.addEventListener('click', () => choose(value));
    modes.append(button);
  }
  head.append(modes);
  if (mode === 'raw') {
    const filter = element('span', 'log-filter');
    for (const [value, label] of [['all', say('logs/all')], ['warn', say('logs/warn')], ['error', say('logs/error')]]) {
      const button = element('button', value === level ? 'on' : null, label);
      button.addEventListener('click', () => choose(value));
      filter.append(button);
    }
    head.append(filter);
  }
  panel.append(head);
  if (mode === 'activity') {
    const rows = activityRows(logs, names, tagged);
    if (rows.length === 0) panel.append(element('p', 'faint', say('logs/none')));
    panel.append(...rows);
    return panel;
  }
  if (logs.length === 0) panel.append(element('p', 'faint', say('logs/none')));
  for (const line of logs) {
    const row = element('div', 'log');
    const time = element('span', 'faint', line.time);
    time.title = line.date;
    row.append(time, element('span', `lv lv-${line.level.toLowerCase()}`, line.level), logText(line.text));
    row.title = line.text;
    panel.append(row);
  }
  return panel;
}

/** 日志的一行字照核心的写法拆开上色（施工 I-6 补三，2026-10-20 项目主人定）：「这件事」正常色，后面的 `键=` 淡一档，值正常色。
 *  值里有空白、引号的核心加了双引号，照原样画。 */
function logText(text) {
  const shown = element('span', 'log-text');
  const at = text.search(/ [a-z_][a-z0-9_.]*=/);
  shown.append(element('span', 'log-msg', at < 0 ? text : text.slice(0, at)));
  if (at < 0) return shown;
  for (const field of text.slice(at).matchAll(/\s*([a-z_][a-z0-9_.]*)=("(?:[^"\\]|\\.)*"|\S+)/g)) {
    shown.append(document.createTextNode(' '), element('span', 'log-key', `${field[1]}=`), element('span', 'log-value', field[2]));
  }
  return shown;
}

/** 切换器头上的「全部」：几个连接、几个连着（施工 I-85 中）。 */
function everyTile(connections, on) {
  const card = element('div', on ? 'conn-tile pick on' : 'conn-tile pick');
  const face = element('span', 'avatar every');
  face.append(icon('i-link'));
  const grow = element('div', 'grow');
  const connected = connections.filter((one) => one.connected).length;
  grow.append(
    element('b', null, say('conn/all')),
    element('div', 'soft', say('conn/all-line', { count: connections.length, connected })),
  );
  card.append(face, grow);
  card.addEventListener('click', () => chooseConnection(EVERY));
  return card;
}

/** 一个连接的卡：`picking` 不空的是切换器上的一张（`on` 是选中着），点了选中它。 */
function tile(one, request, picking) {
  const shown = platform(one.platform);
  let className = one.connected ? 'conn-tile' : 'conn-tile off';
  if (picking) className += picking.on ? ' pick on' : ' pick';
  const card = element('div', className);
  const bot = element('span', 'bot');
  const face = element('span', `avatar pf-face pf-${one.platform}`);
  face.append(mark(shown));
  if (one.avatar) picture(face, one.avatar, () => (face.className = 'avatar'));
  const badge = element('span', `pf-badge pf-${one.platform}`);
  badge.append(mark(shown));
  bot.append(face, badge);
  const grow = element('div', 'grow');
  const line = one.implementation
    ? say('conn/line', { platform: shown.name(), implementation: one.implementation, version: one.version })
    : shown.name();
  const state = element('div', 'faint');
  state.append(element('span', one.connected ? 'dot on' : 'dot off'), document.createTextNode(` ${since(one)}`));
  grow.append(element('b', null, one.name || one.bot), element('div', 'soft', line), state);
  const gear = element('button', 'icon-button');
  gear.title = say('settings/open');
  gear.append(icon('i-gear'));
  gear.addEventListener('click', (event) => {
    event.stopPropagation();
    // 连接设置照这个连接的平台问、存（施工 I-85 中）。
    openSettings((method, params) => request(method, { ...params, platform: one.platform }), one);
  });
  card.append(bot, grow, gear);
  if (picking) card.addEventListener('click', () => chooseConnection(one.id));
  return card;
}

/** 「号 · 已连多久」，断开了的「号 · 断开多久，在重连」。 */
function since(one) {
  const minutes = Math.floor((Date.now() - one.since) / 60000);
  const hours = Math.floor(minutes / 60);
  const fields = { bot: one.bot, n: minutes };
  if (!one.connected) {
    if (minutes < 1) return say('conn/gone-now', fields);
    if (minutes < 60) return say('conn/gone-minutes', fields);
    if (hours < 24) return say('conn/gone-hours', { ...fields, n: hours });
    return say('conn/gone-days', { ...fields, n: Math.floor(hours / 24) });
  }
  if (minutes < 1) return say('conn/since-now', fields);
  if (minutes < 60) return say('conn/since-minutes', fields);
  if (hours < 24) return say('conn/since-hours', { ...fields, n: hours });
  return say('conn/since-days', { ...fields, n: Math.floor(hours / 24) });
}
