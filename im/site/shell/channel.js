// 页面和宿主说话的通道（施工 I-5 上的 /channel，I-6 上起页面经它取东西）：WebSocket，第一帧出示会话号，宿主回账号；之后一问一答，
// 宿主手里的状态变了推 {"event": "changed"}（页面照它再问，不轮询）。
//
// - 关闭帧 4401：会话号不认了（登出了、im 重启过），交给 onExpired，不再连。
// - 别的断开：交给 onLost，过一会儿再连（1 秒起、每次翻倍、最多 10 秒）；连上了交给 onOpen。在等回答的都算失败。

/** 用会话号 `session` 开通道；`handlers` 是 onOpen(account)、onEvent(name)、onLost()、onExpired()。 */
export function openChannel(session, handlers) {
  let socket;
  let next = 1;
  let delay = 1000;
  let closed = false;
  const waiting = new Map();

  const start = () => {
    socket = new WebSocket(`ws://${location.host}/channel`);
    let greeted = false;
    socket.addEventListener('open', () => socket.send(JSON.stringify({ session })));
    socket.addEventListener('message', (event) => {
      let message;
      try {
        message = JSON.parse(event.data);
      } catch {
        return;
      }
      if (!greeted) {
        greeted = true;
        delay = 1000;
        handlers.onOpen?.(message.account);
        return;
      }
      if (message.event) {
        handlers.onEvent?.(message.event);
        return;
      }
      const asked = waiting.get(message.id);
      if (!asked) return;
      waiting.delete(message.id);
      if (message.error) asked.reject(message.error.reason);
      else asked.resolve(message.result);
    });
    socket.addEventListener('close', (event) => {
      for (const asked of waiting.values()) asked.reject('closed');
      waiting.clear();
      if (closed) return;
      if (event.code === 4401) {
        closed = true;
        handlers.onExpired?.();
        return;
      }
      handlers.onLost?.();
      setTimeout(start, delay);
      delay = Math.min(delay * 2, 10000);
    });
  };
  start();

  return {
    /** 调宿主的方法 `method`：交回 result，拒了、断了的交回原因。 */
    request(method, params = {}) {
      return new Promise((resolve, reject) => {
        if (socket.readyState !== WebSocket.OPEN) {
          reject('closed');
          return;
        }
        const id = next++;
        waiting.set(id, { resolve, reject });
        socket.send(JSON.stringify({ id, method, params }));
      });
    },
    /** 不再连。 */
    close() {
      closed = true;
      socket.close();
    },
  };
}
