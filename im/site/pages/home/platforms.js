// 各个平台在页面上的样子（施工 I-6 中，设计 01 第八节第 2 条「后面的步子照驱动声明的连接设置一步步画」）：驱动报它的配置项
// （Driver::settings，OneBot 是 listen、token），这里写每个平台怎么画：角标、名字、新建连接的几步、连接设置的几格、要填到那边的
// 地址怎么拼（照宿主 status 交的 listen、path）。加平台就加一项。
import { element, say } from '../../shell/i18n.js';

export const PLATFORMS = {
  qq: {
    badge: 'Q',
    // 平台的标志：雪碧图里的编号（施工 I-6 补二，2026-10-19 项目主人：原来的字母「Q」要优化）。
    logo: 'logo-qq',
    name: () => say('platform/qq'),
    // 会话页（施工 I-7 上修）：这个平台有哪几种会话，号怎么叫（设计 01 第八节第 7 条：叫法由驱动给），新建、浮窗里说这个平台
    // 自己的规矩（QQ：私聊白名单里的人加 AI 好友自动通过）。没写的照通用的（venueKinds）。
    venues: {
      group: { id: () => say('platform/qq-group-id') },
      private: {
        id: () => say('platform/qq-user-id'),
        searchHint: () => say('platform/qq-private-search-hint'),
        newHint: () => say('platform/qq-new-private-hint'),
        listedHint: () => say('platform/qq-listed-hint'),
      },
    },
    driver: () => say('platform/qq-driver'),
    address: (status) => `ws://127.0.0.1:${status.listen}${status.path}`,
    // 新建连接：选了平台以后的几步。address 是照着填地址的那一步，secret 是粘贴一个密钥、存了就连。
    steps: [
      {
        kind: 'address',
        title: () => say('wizard/qq-address-title'),
        text: () => say('wizard/qq-address-text'),
        next: () => say('wizard/filled'),
      },
      {
        kind: 'secret',
        name: 'token',
        title: () => say('wizard/qq-token-title'),
        text: () => say('wizard/qq-token-text'),
        label: () => say('wizard/token'),
        placeholder: () => say('wizard/token-placeholder'),
        next: () => say('wizard/save-connect'),
      },
    ],
    waiting: () => say('wizard/qq-wait-text'),
    node: () => say('wizard/qq-node'),
    // 连接设置的几格。
    settings: [
      { kind: 'port', name: 'listen', label: () => say('settings/qq-listen'), hint: () => say('settings/qq-listen-hint') },
      { kind: 'secret', name: 'token', label: () => say('settings/qq-token'), hint: () => say('settings/qq-token-hint') },
      { kind: 'address', label: () => say('settings/qq-address') },
    ],
  },
};

/** 平台 `shown` 的标志：有图形的画图形（白色，底色由外面的 class 定），没有的写字母。 */
export function mark(shown) {
  if (!shown.logo) return document.createTextNode(shown.badge);
  const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
  svg.setAttribute('class', 'logo');
  svg.setAttribute('aria-hidden', 'true');
  const use = document.createElementNS('http://www.w3.org/2000/svg', 'use');
  use.setAttribute('href', `#${shown.logo}`);
  svg.append(use);
  return svg;
}

/** 会话的几种通用的叫法：平台没写的照这里。 */
const GENERIC = {
  group: {
    id: () => say('venues/group-id'),
    searchHint: () => say('create/group-search-hint'),
    newHint: () => say('create/new-group-hint'),
  },
  private: {
    id: () => say('venues/user-id'),
    searchHint: () => say('create/private-search-hint'),
    newHint: () => say('create/new-private-hint'),
    listedHint: () => say('venue/listed-hint'),
  },
};

/** 平台 `id` 上有哪几种会话（group、private），每种的叫法；平台没写会话的两种都有。 */
export function venueKinds(id) {
  const declared = platform(id).venues || { group: {}, private: {} };
  return Object.fromEntries(Object.entries(declared).map(([type, words]) => [type, { ...GENERIC[type], ...words }]));
}

/** 平台 `id` 的样子；不认识的照名字凑一个最简单的。 */
export function platform(id) {
  return PLATFORMS[id] || { badge: id.slice(0, 1).toUpperCase(), name: () => id, driver: () => '', steps: [], settings: [] };
}
