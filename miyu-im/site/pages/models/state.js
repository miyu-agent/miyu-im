// 模型页读的、存的（设计 09，施工 I-149；照网页软件 dialog.js 的 load、saveMany 和 model-bits.js 的 refreshProvider）：model.list、
// config.schema、config.get 一起读；改配置一律一条 config.set（几项全收或者全不收），成了重读，拒了交回给人看的一句：冲突的先重读、
// 写「别处改过了，现在是 X」，写错的把几处连起来。不订阅核心的推送（设计 09 第二节第 3 条）：打开这一页、自己存完都重读。
import { say } from '../../shell/i18n.js';
import { core, refusalText } from '../../shell/core.js';
import { LAYER } from './config.js';

/** 模型页的状态：`request` 调宿主，读完、存完调 `changed()` 重画。 */
export function modelState(request, changed) {
  const state = {
    request,
    /** model.list 的回应、config.schema、config.get {all}；没读到的是 null。 */
    models: null,
    schema: null,
    got: null,
    /** 读不到时的原因。 */
    failed: null,
    /** 正在拉模型列表的那一家。 */
    loadingProvider: null,
    /** 这一页里拉过模型列表的几家（没有模型的一家只自动拉一次）。 */
    refreshed: new Set(),

    async load() {
      try {
        const [models, schema, got] = await Promise.all([
          core(request, 'model.list'),
          core(request, 'config.schema'),
          core(request, 'config.get', { all: true }),
        ]);
        Object.assign(state, { models, schema, got, failed: null });
      } catch (error) {
        state.failed = refusalText(error);
      }
      changed();
    },

    /** 一条 config.set 改几项：成了重读、交回 null，拒了交回给人看的一句。 */
    async saveMany(changes) {
      if (!changes.length) return null;
      try {
        await core(request, 'config.set', { layer: LAYER, changes });
      } catch (error) {
        if (error.reason === 'config_conflict' || error.data?.current) {
          await state.load();
          const current = error.data?.current;
          const now = current && 'value' in current ? JSON.stringify(current.value) : say('models/unset');
          return say('models/conflict', { value: now });
        }
        return refusalText(error);
      }
      await state.load();
      return null;
    },

    /** 重读一家的模型（`refresh` 的请核心先去供应商那边拉一遍）：读的时候那一块写「正在读模型列表」。 */
    async refreshProvider(id, refresh) {
      state.refreshed.add(id);
      state.loadingProvider = id;
      changed();
      try {
        const got = await core(request, 'model.list', { provider: id, ...(refresh ? { refresh: true } : {}) });
        const fresh = got?.providers?.find((one) => one.id === id);
        if (fresh && state.models) {
          state.models.providers = state.models.providers.map((one) => (one.id === id ? fresh : one));
        }
      } catch {
        // 读不到的照旧显示原来的。
      } finally {
        state.loadingProvider = null;
        changed();
      }
    },
  };
  return state;
}
