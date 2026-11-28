## 施工单 I-276：Windows 的程序不要 VC++ 运行库

状态：做完了（2026-11-28）。

### 起因

核心 CRT-1（2026-11-28）查出 Windows 包里的程序都要 `VCRUNTIME140.dll`。这个 DLL 是装 VC++ 2015–2022 运行库时放进系统的，Windows 自己不带，没装过的机器上报「找不到 VCRUNTIME140.dll」、起不来。GitHub 的 Windows 机器本来装着运行库，所以 CI 一直看不出来。im 自己发 Windows 包，core1 转来照核心一样改，跟 0.3.6 一起发。

### 改了什么

- `.cargo/config.toml` 给 `x86_64-pc-windows-msvc` 加 `-C target-feature=+crt-static`，C 运行库静态链进程序，cc 编的 SQLite 也换成静态的运行库。发版的 `release.yml` 在仓库里编，照样读这份配置。
- CI 的 Windows 那台在门禁以后读 `miyu-im.exe`、`miyu-im-sponsor.exe`，里面有 `vcruntime140`、`msvcp140` 的字样就报。门禁红了也照查。

### 怎么验

- 只加检查、不改配置时，Windows 上报红：`target/debug/miyu-im.exe` 要了运行库（CI 运行 46134192404）。
- 加上配置以后三台 CI 全绿，这一步也过。
