//! 页面服务听哪个地址（设计 11 第一节第 3 条，施工 I-167）：配置 `im.web.listen`，出厂 `127.0.0.1` 只给本机；设置页「向局域网开放」
//! 打开写 `0.0.0.0`，关了写回 `127.0.0.1`。照网页软件的 `web.listen`（miyu-web `listen.rs`）。
//!
//! - 只认两种写法：本机 `127.0.0.1`、局域网 `0.0.0.0`。没写的、别的照本机（页面上只有这一个开关，不给写别的地址）。
//! - 局域网地址（施工 I-193 起照网页软件 W-1135）：听 `0.0.0.0` 时照这台电脑的网卡全列、带网卡名（`if-addrs`），回环、链路本地
//!   的不列，一个地址只列一次；设置页照第一个画二维码，别的写在下面。读不到网卡的退回问系统往外走用哪个地址（UDP 套接字连一个
//!   文档用的地址 192.0.2.1，不发包，读本地地址），不知道网卡名；都问不到的是空的。原来只照往外的那个，开着代理的 TUN 网卡时只给了
//!   一个 198.18 开头的虚拟地址（项目主人 11-10 在网页软件那边撞见）。
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

/// 听 `ip` 时局域网里打得开的地址和网卡名（模块开头）：只听本机的是空的，听某一个地址的就是它（不知道网卡名）。
pub(super) fn lan_addresses(ip: IpAddr) -> Vec<(Option<String>, IpAddr)> {
    if ip.is_loopback() {
        return Vec::new();
    }
    if !ip.is_unspecified() {
        return vec![(None, ip)];
    }
    let cards: Vec<(String, IpAddr)> = if_addrs::get_if_addrs()
        .map(|all| {
            all.into_iter()
                .map(|card| (card.name.clone(), card.ip()))
                .collect()
        })
        .unwrap_or_default();
    pick(&cards, outward())
}

/// 网卡 `cards`（名字、地址）里别的设备可能连得进来的 IPv4 地址，照网卡的先后、一个地址一次（模块开头）；一个都没有的照往外的那个
/// `outward`（不知道网卡名）。
pub(super) fn pick(
    cards: &[(String, IpAddr)],
    outward: Option<IpAddr>,
) -> Vec<(Option<String>, IpAddr)> {
    let mut got: Vec<(Option<String>, IpAddr)> = Vec::new();
    for (name, ip) in cards {
        if usable(*ip) && !got.iter().any(|(_, seen)| seen == ip) {
            got.push((Some(name.clone()), *ip));
        }
    }
    if got.is_empty()
        && let Some(ip) = outward.filter(|ip| usable(*ip))
    {
        got.push((None, ip));
    }
    got
}

/// 别的设备可能连得进来：IPv4（听的是 `0.0.0.0`），不是回环、没指定、链路本地（`169.254.0.0/16`）。
fn usable(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => !(v4.is_loopback() || v4.is_unspecified() || v4.is_link_local()),
        IpAddr::V6(_) => false,
    }
}

/// 这台电脑往外的那个地址：拿不到的、是回环的、没指定的是空的。
fn outward() -> Option<IpAddr> {
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0)).ok()?;
    socket.connect(OUTWARD).ok()?;
    let local = socket.local_addr().ok()?.ip();
    (!local.is_loopback() && !local.is_unspecified()).then_some(local)
}

#[cfg(test)]
mod tests;
