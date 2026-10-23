// 平台会话页（施工 I-34 下，设计 01 第八节第 9 条；2026-10-23 项目主人定：场所会话只读，看的地方放进 im 自己的后台页，按连接分组
// 列会话，点进去看只读的记录，管理员能删）。#sessions 是列表（list.js），#sessions/<会话编号> 是一个会话的记录（record.js）。
//
// - 宿主每来一条消息都推一声「变了」（外壳调 update）：列表最多十秒重问一次（问群名要问 NapCat），记录最多一秒半重读最新一页。
import { mountList } from './list.js';
import { mountRecord } from './record.js';

/** 在 `view` 里起平台会话页：`request` 调宿主的方法，`path` 是会话编号（空的是列表）。交回 update(connections)、dispose()。 */
export function mountSessions(view, { request, path }) {
  const shown = path ? mountRecord(view, { request, session: path }) : mountList(view, { request });
  return {
    update(connections) {
      shown.update(connections);
    },
    dispose() {
      shown.dispose();
    },
  };
}

/** `wait` 毫秒以内最多跑一次 `run`：期间叫过的，到点补跑一次。交回叫它的那一个和停下的那一个。 */
export function throttled(wait, run) {
  let last = 0;
  let timer = 0;
  const call = () => {
    const now = Date.now();
    if (now - last >= wait) {
      last = now;
      run();
    } else if (!timer) {
      timer = setTimeout(() => {
        timer = 0;
        last = Date.now();
        run();
      }, wait - (now - last));
    }
  };
  return { call, stop: () => clearTimeout(timer) };
}
