## 施工单 LW-IM-9：发版接上排函数

状态：写好了（2026-11-23），还没在 CI 的机器上跑过。

### 为什么

LW-IM-8 做了 `cargo xtask order`（照执行的先后排函数，空闲时常驻的代码 8.3 → 3.4 MB），发版还照旧直接编。

### 怎么改

1. `.github/workflows/release.yml` 交给 `miyu-agent/packages` 的 `pack.yml` 的构建脚本：Linux x86_64 那一份（`$TARGET` 是
   `linux-x86_64`）装 `qemu-user`，编程序那一句换成 `cargo xtask order`；它失败（录不出、对不上、排不上）这一步就失败，不打、不发。
   macOS、Windows 照旧 `cargo build --release`。`pack.yml` 不改。
2. 只验排函数、不打包的手动入口：`ci.yml` 多一个 job `order`，只在手动跑（workflow_dispatch）时跑，Ubuntu 24.04 上装 `qemu-user`、
   跑 `cargo xtask order`，清单放进产物；最多 60 分钟。不进每次推送都跑的那一组（它要十来分钟）。
3. `pack.yml` 的超时：只读地看过，`pack` 这个 job 没写 `timeout-minutes`，照 GitHub 的默认六个小时，多十来分钟够，不用改。

图纸：`docs/blueprint/onebot.md` 第一条「在哪」多 `.github/workflows/release.yml`、`ci.yml` 的 `order` 一行。

### 验收

推分支以后手动跑一次 `ci.yml`（`order` 那个 job 过、产物里有清单），再在 `step/**` 上手动跑一次 `release.yml` 试打（Linux x86_64 那一份的
程序是排过的）。这两样这次都没有跑过。

### 风险

- CI 的机器上集成测试在模拟器里比本机慢，个别等时刻的测试可能没过：只影响录到多少，`order` 照样往下走；一段一份日志都没有才失败。
- `cargo xtask order` 多编一遍 release 的集成测试，Linux 那一份发版多十来分钟。
