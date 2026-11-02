## 施工单 I-120：测试夹具的网页端口不撞 NapCat 端口

状态：做完了（2026-11-02；I-109 本机门禁偶发红查出来的）。

### 目的

I-109 的门禁第一次跑 `site_today::the_home_page_counts_today_and_shows_the_log` 红了，单独跑三次都过。日志里桥听 NapCat 的端口和页面服务的端口是同一个号（34867），页面开不了。根因：测试夹具 `ports_config` 先挑 NapCat 的端口，再另调一次 `free_port()` 挑页面的；`free_port` 绑 `127.0.0.1:0` 拿到号就放掉，两次可能拿到同一个号。

### 怎么走

1. `tests/support/spawning.rs` 多 `other_port(taken)`：挑到的是 `taken` 的再挑。`ports_config_with` 给页面挑端口照它，躲开 NapCat 的。
2. 别的几处挑端口的不改：要么有「被占了换一个再来」（`moved`），要么挑的端口本来就不开（握手里的旧 `im.web`），要么另一个端口挑之前已经绑着（进程里的桥照端口 0 让系统挑，系统不会再给出去）。

### 验收

- 竞态测不出必现，照原因改：同一个号不会再给两个端口。
- 本机门禁、三台 CI 全绿。
