## 施工单 I-57：/model 搜和换分开

状态：做完了。

### 目的

项目主人 10-27 问：在 QQ 里怎么知道有哪些模型？I-53 的 `/model` 只说现在用哪个，`/model <词>` 模糊找到一个就直接换。不知道名字就没法选；模糊找错了还会直接换掉。

项目主人定：别的程序（例如 astrbot）是 `/model` 列出全部、标出正在用的。可我们能用的模型太多，不全列。改成：

- `/model search <关键词>` 模糊搜；
- `/model <确切名字>` 切换。

### 改了什么

- `command/model.rs`：后面的词先分四种（`asked`）：没写、`search`、`默认`、名字。
  - `/model`：说现在用哪个，再说一行怎么用。
  - `/model search <关键词>`：模糊找（同 I-50 的写法），列前 10 个，多的说还有几个，正在用的标「（正在用）」。没写关键词的照先后列，池在前。
  - `/model <名字>`：只认确切的名字，不分大小写。只写了模型那一段（最后一个 `/` 后面），又只有一个对得上的，也算。对不上的说没有，提示用 `/model search` 去搜。
  - `/model 默认`：照旧去掉卡片上的模型。
- 字：`model-usage`、`model-found`、`model-using-mark`、`model-search-none` 是新加的，`model-none` 改了说法，`model-many` 删了。三种语言。
- 蓝图 `onebot.md` 的 `/model` 那段改写。

### 测试

- `command/model/tests.rs`：
  - 四种分得对；`searching` 不算 `search`。
  - 确切名字：整个一样的；只写模型那一段、只有一个的；两个供应商都有的不算；模糊的不算。
  - 搜：先后不论；最多 10 个、数出剩下的；没写关键词的照原来的先后。
- `tests/commands.rs` 改写（真核心）：
  - `/model` 带怎么用；
  - `/model lite` 不换，提示去搜；
  - `/model search lite` 列出 `@lite`；
  - `/model @lite` 换上；再搜，标「（正在用）」；
  - `/model 默认` 去掉。
- 变异：名字照模糊找第一个就换，这条转红。

### 验收

本机门禁 `cargo xtask check` 全过，CI 三台全绿。部署后在 QQ 私聊依次打：

1. `/model`
2. `/model search flash`
3. `/model <上一步列出来的某个名字>`
4. `/model search flash`（看那个名字后面有没有「（正在用）」）
5. `/model 默认`
