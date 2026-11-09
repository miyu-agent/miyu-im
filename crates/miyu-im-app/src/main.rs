//! 程序 `miyu-im`（`onebot.md` 第一条「对外的样子」）：宿主的入口（`miyu_im::program`）交各个平台的驱动（施工 I-84；施工 I-183 上从
//! OneBot 的 crate 挪出来）。接新平台：这里多列一个驱动。

fn main() -> std::process::ExitCode {
    miyu_im::program::main(vec![
        std::sync::Arc::new(miyu_im_onebot::OneBot),
        std::sync::Arc::new(miyu_im_telegram::Telegram),
    ])
}
