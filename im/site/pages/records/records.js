// 消息记录页（施工 C-1 中下，设计 04 第一节第 2、3 条，设计 01 第八节第 9 条；「平台会话」改名）：读 im 自己的聊天记录库，不经核心。
// #records 是场所的列表（venues.js），#records/<场所> 是一个场所的记录（venue.js），#records/<场所>/<行号> 从那一条往回看（搜到的点进来）。
//
// - 宿主每来一条消息都推一声「变了」（外壳调 update）：列表最多十秒重问一次，记录最多一秒半重读最新一页。
import { mountVenues } from './venues.js';
import { mountVenue } from './venue.js';

/** 在 `view` 里起消息记录页：`request` 调宿主的方法，`path` 是场所（空的是列表），后面可以带 `/<行号>`。交回 update(connections)、dispose()。 */
export function mountRecords(view, { request, path }) {
  const cut = path.lastIndexOf('/');
  const venue = cut > 0 ? path.slice(0, cut) : path;
  const at = cut > 0 ? Number(path.slice(cut + 1)) || null : null;
  const shown = venue ? mountVenue(view, { request, venue, at }) : mountVenues(view, { request });
  return {
    update(connections) {
      shown.update(connections);
    },
    dispose() {
      shown.dispose();
    },
  };
}
