//! 程序 `miyu-im`（`onebot.md` 第一条「对外的样子」）：宿主的入口（`miyu_im::program`）交 OneBot 一个驱动（施工 I-84）。接新平台：
//! 这里多列一个驱动。

fn main() -> std::process::ExitCode {
    miyu_im::program::main(vec![std::sync::Arc::new(miyu_im_onebot::OneBot)])
}
