## 施工单 I-177：新建连接粘贴 Token 误报「这个端口已经被占用了」

状态：做完了（2026-11-08；项目主人在新数据根上实测碰到，core1 转来）。

### 为什么

新数据根上配置里先写了 `[im.qq] token = {secret = "im-qq-token"}`，密钥却还没有。im 起来时照没有令牌听 8301（日志「no token yet」），驱动报 8301 `token: none`。向导只把设了 Token 的端口当用着的，照驱动报的把 8301 填进地址；粘贴 Token 点「保存并连接」时 `connection.add` 照配置读连接表，原来那一项 Token 是引用、算一个连接，端口重了回 `bad_params`。页面把所有 `bad_params` 都说成「这个端口已经被占用了」。

存 Token 这一步本身不该挑端口：端口在填地址那一步定了，正是 im 自己没令牌听着的那一个；im 也没把自己的端口当成别人的，是表里那一项（Token 解不出来）和驱动报的对不上。

### 怎么改

1. `connection.add`：表里有这个端口、驱动却报它还没有令牌的，不算重，给那一项换上新 Token（写了地址的照写的改），不再新加一项。
2. 端口重了的（和表里的、和网页自己的），`connection.add`、`connection.edit` 回 `port_taken`；写错的（Token 没写、找不到、端口超出范围、地址不是 IP）照旧 `bad_params`。
3. 页面：向导、连接齿轮照 `port_taken` 说「端口被占用」，别的 `bad_params` 说存不成（地址的照旧说要写 IP）。

### 验收

- 回归测试 `site_connections::the_first_connection_fills_a_port_listening_without_a_token`：配置写一个找不到的密钥引用、桥没令牌听着，在这个端口 `connection.add` 存得成、表里一项、换上新密钥；退回修以前报红（`bad_params`）。
- `the_page_adds_edits_and_removes_connections` 端口重了的四种回 `port_taken`，退回修以前报红。
- 本机门禁、三台 CI 全绿。
