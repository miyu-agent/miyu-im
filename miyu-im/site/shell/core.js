// 经宿主的转发口调核心（设计 09 第二节第 1 条，施工 I-145）：人格、预设、模型三页用。宿主照白名单原样转，交回 { result } 或
// { refused: { reason, message } }；这里拆开：接了的交回 result，核心拒了的抛 Refused（带原因、原话），im 自己拒了的（不在白名单、
// 通道断了）也抛 Refused，原因照通道给的。
import { say } from './i18n.js';

/** 核心或宿主拒了：`reason` 照它写给人看的字，`message` 是原话（可能空），`data` 是核心拒绝里带的（宿主转了才有）。 */
export class Refused extends Error {
  constructor(reason, message, data) {
    super(message || reason);
    this.reason = reason;
    this.said = message || '';
    this.data = data ?? null;
  }
}

/** 用 `request`（调宿主的方法）调核心的 `method`。 */
export async function core(request, method, params = {}) {
  let answer;
  try {
    answer = await request('core.call', { method, params });
  } catch (reason) {
    throw new Refused(String(reason), '');
  }
  if (answer.refused) throw new Refused(answer.refused.reason, answer.refused.message, answer.refused.data);
  return answer.result;
}

/** 拒了的写给人看的一句：核心照连接的语言给人看的那一句（data.message），写错的几处（data.problems 的 message 连起来），再是原话，都没有的
 *  照原因写一句通用的。 */
export function refusalText(error) {
  if (error instanceof Refused) {
    if (typeof error.data?.message === 'string' && error.data.message) return error.data.message;
    const problems = Array.isArray(error.data?.problems) ? error.data.problems.map((one) => one?.message).filter(Boolean) : [];
    if (problems.length) return problems.join('；');
    if (error.said) return error.said;
  }
  return say('failed/refused', { reason: error?.reason || String(error) });
}
