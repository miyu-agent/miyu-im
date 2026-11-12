// 实时日志的推送（设计 10，施工 I-161）：通道推来的 {"event": "watch", …} 交给开着「实时日志」那一页的人；通道断了重连上，叫它们
// 再开始看一次（宿主那一头照通道记着看什么，断了就没了）。

const listeners = new Set();

/** 听实时日志：`onPush(message)` 收推来的一帧，`onReopen()` 在通道重连上以后。交回不再听的函数。 */
export function listenWatch(onPush, onReopen) {
  const one = { onPush, onReopen };
  listeners.add(one);
  return () => listeners.delete(one);
}

/** 通道推来一帧实时日志。 */
export function pushWatch(message) {
  for (const one of listeners) one.onPush(message);
}

/** 通道重连上了。 */
export function reopenWatch() {
  for (const one of listeners) one.onReopen?.();
}
