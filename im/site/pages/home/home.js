// 首页（设计 01 第八节第 2 条，施工 I-6 上起）：一个连接都没有的正中一个「新建连接」；有连接的「连接」一栏，一个机器人一张卡：
// 头像（页面服务转的，取不到的写平台的字母）、平台的角标、名字（没问到的写号）、平台和实现、号和连了多久；断开了的卡留着，写断开
// 多久、在重连（施工 I-6 中）。齿轮打开连接设置，「新建连接」打开引导。下面「今天」四个数（收到、回复、只记录、没发出去）和最近
// 的运行日志（只读，能按级别筛），宿主推了变了跟着连接一起再问（施工 I-6 下）。连了多久每分钟照本机的钟重画一次（不问宿主）。
import { element, icon, say } from '../../shell/i18n.js';
import { mark, platform } from './platforms.js';
import { openSettings } from './settings.js';
import { openWizard } from './wizard.js';

/** 在 `view` 里起首页：`request` 调宿主的方法。交回 update(connections)、dispose()。 */
export function mountHome(view, { request }) {
  let connections = [];
  let wizard = null;
  let today = null;
  let logs = [];
  let level = 'all';
  const refresh = async () => {
    try {
      [today, logs] = await Promise.all([
        request('today'),
        request('logs', { level, limit: 30 }).then((answer) => answer.logs),
      ]);
    } catch {
      return;
    }
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
    for (const one of connections) grid.append(tile(one, request));
    const todayHead = element('div', 'section-head');
    todayHead.append(element('h2', null, say('home/today')));
    view.replaceChildren(head, section, grid, todayHead, numbers(today), logPanel(logs, level, (chosen) => {
      level = chosen;
      refresh();
    }));
  };
  const timer = setInterval(draw, 60000);
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
    },
  };
}

/** 今天的四个数。还没问到的写「—」。 */
function numbers(today) {
  const grid = element('div', 'home-grid');
  for (const [key, label] of [
    ['received', say('home/received')],
    ['replied', say('home/replied')],
    ['recorded', say('home/recorded')],
    ['undelivered', say('home/undelivered')],
  ]) {
    const panel = element('div', 'panel stat');
    const shown = today ? Number(today[key] ?? 0).toLocaleString() : '—';
    panel.append(element('b', null, shown), element('span', null, label));
    grid.append(panel);
  }
  return grid;
}

/** 最近的运行日志：头上按级别筛（全部、提醒、出错），一行一条，新的在前。 */
function logPanel(logs, level, choose) {
  const panel = element('div', 'panel logs');
  const head = element('h3', null, say('home/logs'));
  const filter = element('span', 'log-filter');
  for (const [value, label] of [['all', say('logs/all')], ['warn', say('logs/warn')], ['error', say('logs/error')]]) {
    const button = element('button', value === level ? 'on' : null, label);
    button.addEventListener('click', () => choose(value));
    filter.append(button);
  }
  head.append(filter);
  panel.append(head);
  if (logs.length === 0) panel.append(element('p', 'faint', say('logs/none')));
  for (const line of logs) {
    const row = element('div', 'log');
    const time = element('span', 'faint', line.time);
    time.title = line.date;
    row.append(time, element('span', `lv lv-${line.level.toLowerCase()}`, line.level), element('span', 'log-text', line.text));
    row.title = line.text;
    panel.append(row);
  }
  return panel;
}

function tile(one, request) {
  const shown = platform(one.platform);
  const card = element('div', one.connected ? 'conn-tile' : 'conn-tile off');
  const bot = element('span', 'bot');
  const face = element('span', `avatar pf-face pf-${one.platform}`);
  face.append(mark(shown));
  if (one.avatar) {
    const picture = element('img');
    picture.alt = '';
    picture.src = one.avatar;
    picture.addEventListener('load', () => {
      face.className = 'avatar';
      face.replaceChildren(picture);
    });
  }
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
  gear.addEventListener('click', () => openSettings(request, one));
  card.append(bot, grow, gear);
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
