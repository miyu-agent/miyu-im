## 施工单 LW-IM-4：程序的重定位表打包，空闲少占它那一块

状态：做完了（2026-11-23）。

### 为什么

`miyu-im` 是位置无关的程序：起来时动态链接器把整张重定位表（`.rela.dyn`）读一遍，读过的页一直留在内存里。2026-11-23 量过，这一张 400 KB，在每一种状态下都整张常驻。核心 LW-2 已经把自己的打包成 RELR，只剩二十来 KB。

### 怎么改

1. 新的 `crates/miyu-im-app/build.rs`：Linux、x86_64、glibc 的 release 构建给 `miyu-im` 这一个程序加 `-z pack-relative-relocs`（同核心 `crates/miyu/build.rs`）。别的系统、debug、测试用的程序 `miyu-im-test-hook` 不加。
2. RELR 要 glibc 2.36 以上：im 跟着核心发，最低要的核心在 Linux 上的基线是 2.39（核心的设计 12），发版的机器是 Ubuntu 24.04。打出来的程序多要一个 `GLIBC_ABI_DT_RELR`。

图纸：`docs/blueprint/onebot.md` 第一条「在哪」多 `build.rs` 一行。

### 测试

- 门禁全过，原有的测试一条没改。
- `readelf -S`：`.rela.dyn` 412 KB → 4.5 KB，多一段 `.relr.dyn` 4.9 KB；`readelf -V` 多 `GLIBC_ABI_DT_RELR`。

### 量（沙箱，空闲 30 秒；改前改后各三遍交替着量，PSS，kB）

| 状态 | 改前 | 改后 |
|---|---|---|
| 没配平台 | 12459 / 12493 / 12683 | 12162 / 12228 / 12169 |
| 接上假 NapCat | 13302 / 13246 / 13230 | 12898 / 13042 / 12769 |

少的那一块在程序的只读段里（1.2–1.35 MB → 0.83–0.96 MB）。

### 不做的

- aarch64、musl、macOS、Windows：链接器不认或者另是一回事，没有量过。
