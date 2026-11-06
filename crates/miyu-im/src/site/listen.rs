//! 页面服务听哪个地址（设计 11 第一节第 3 条，施工 I-167）：配置 `im.web.listen`，出厂 `127.0.0.1` 只给本机；设置页「向局域网开放」
//! 打开写 `0.0.0.0`，关了写回 `127.0.0.1`。照网页软件的 `web.listen`（miyu-web `listen.rs`）。
//!
//! - 只认两种写法：本机 `127.0.0.1`、局域网 `0.0.0.0`。没写的、别的照本机（页面上只有这一个开关，不给写别的地址）。
//! - 局域网地址：听 `0.0.0.0` 时问系统往外走用哪个地址（UDP 套接字连一个文档用的地址 192.0.2.1，不发包，读本地地址），设置页照它
//!   写网址、画二维码；问不到的是空的。
//! - 开着局域网时，`Host` 写的是 IP 的（局域网里的手机照地址打开）也收；写域名的照旧只认本机的三种写法（挡 DNS 重绑定：别的网站
//!   的域名解析到这台机器时，浏览器带的是那个域名）。

use std::net::{IpAddr, Ipv4Addr, UdpSocket};

use serde_json::{Map, Value};

/// 配置里的名字（`im.web.listen`）。
pub(super) const LISTEN: &str = "listen";

/// 只给本机。
pub(super) const LOCAL: &str = "127.0.0.1";

/// 向局域网开放。
pub(super) const LAN: &str = "0.0.0.0";

/// 往外走问地址时连的那一个（RFC 5737 文档用的地址，不发包）。
const OUTWARD: (Ipv4Addr, u16) = (Ipv4Addr::new(192, 0, 2, 1), 9);

/// 配置要听的地址：`0.0.0.0` 是局域网，别的都是本机。
pub(super) fn wanted(config: &Map<String, Value>) -> IpAddr {
    match config.get(LISTEN).and_then(Value::as_str) {
        Some(LAN) => IpAddr::V4(Ipv4Addr::UNSPECIFIED),
        _ => IpAddr::V4(Ipv4Addr::LOCALHOST),
    }
}

/// 页面上存的值查得过没有：只收两种写法。
pub(super) fn valid(value: &str) -> bool {
    value == LOCAL || value == LAN
}

/// 听 `ip` 时局域网里打得开的地址：只听本机的、问不到的是空的。
pub(super) fn lan_address(ip: IpAddr) -> Option<IpAddr> {
    if ip.is_loopback() {
        return None;
    }
    if !ip.is_unspecified() {
        return Some(ip);
    }
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    socket.connect(OUTWARD).ok()?;
    let local = socket.local_addr().ok()?.ip();
    (!local.is_loopback() && !local.is_unspecified()).then_some(local)
}

#[cfg(test)]
mod tests;
