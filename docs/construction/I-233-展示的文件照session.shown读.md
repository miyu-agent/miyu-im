## 施工单 I-233：展示的文件照 session.shown 读

状态：做完了（2026-11-13），等换钉的核心 CI 绿了一起验。

### 起因

核心 SW-1（core1 11-13 转来，项目主人定：展示不拷进核心、不限大小）：`file.shown` 不再带 `blob`，只记真实路径、大小、修改时间。im 原来照 `blob.get` 读她展示的图和文件发进群，这条路断了。核心另给 `session.shown {session, call, offset?, length?}` 分段读（回应同 `blob.get`，一段最多 512 KiB）；文件改过回 `file_changed`、没了回 `file_gone`。

### 改了什么

1. `core/route/shown.rs`：
   - 认 `file.shown` 要 `path`、`name`、`media_type`、`size` 齐，加上那次工具结果的 `call_id`（`session.shown` 照它找）；
   - 内容照 `session.shown` 在工具结果出现的那条线上一段段读，写成 `base64://…` 交 NapCat（同原来）；
   - 大于 20 MiB 的不读（原来核心替展示设的上限；读进内存、写成 base64 的这条路不收更大的），记 `unshown`，`why` 是 `too_large`；
   - `file_changed`、`file_gone` 照「读不出」记，原话是核心的原因码。
2. 表情包、长文转图照旧读 blob（`read`），不动。
3. 换钉核心 e792056d5（SW-1 以后六台 CI 全绿的那一个）。

### 怎么验

- 单元测试 `shown::tests`：照新样子认，缺路径、大小、调用编号的不认。
- 集成测试（真核心）：`shown`、`telegram_shown` 原有的几条（图片发成图片段、600 KiB 的文本分几段读了传成文件）；新加 `shown::a_file_too_large_to_send_is_noted_and_not_read`（21 MiB 的不传，记 `too_large`）。
- 本机门禁七项。
