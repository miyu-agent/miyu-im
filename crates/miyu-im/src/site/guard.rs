//! 只认本机的地址（施工 I-5 上「怎么走」第 3 条）：`Host` 是本机的三种写法之一、端口是听着的那一个（挡 DNS 重绑定：别的网站的
//! 域名解析到 `127.0.0.1` 时，浏览器带的 `Host` 是那个域名）；带了 `Origin` 的必须是同一个地址（挡别的网站跨站调）。
//! 向局域网开放时（施工 I-167，[`super::listen`]）`Host` 写 IP 的也收：域名照旧只认本机的写法，DNS 重绑定照样挡得住。

/// 本机的三种写法。
const LOCAL: [&str; 3] = ["127.0.0.1", "localhost", "[::1]"];

/// `host`（请求的 `Host` 头）是不是本机的三种写法之一、端口是 `port`。没带的不是。
pub(super) fn local_host(host: Option<&str>, port: u16) -> bool {
    let Some((name, given)) = host.and_then(|host| host.rsplit_once(':')) else {
        return false;
    };
    LOCAL.contains(&name) && given.parse::<u16>().is_ok_and(|given| given == port)
}

/// `host` 是不是这个服务收的：本机的写法都收；`lan`（向局域网开放）时写 IP 的（`192.168.1.2:8302`、`[fe80::1]:8302`）也收。端口
/// 都得是 `port`。
pub(super) fn allowed_host(host: Option<&str>, port: u16, lan: bool) -> bool {
    if local_host(host, port) {
        return true;
    }
    let Some((name, given)) = host.and_then(|host| host.rsplit_once(':')) else {
        return false;
    };
    let name = name
        .strip_prefix('[')
        .and_then(|name| name.strip_suffix(']'))
        .unwrap_or(name);
    lan && name.parse::<std::net::IpAddr>().is_ok()
        && given.parse::<u16>().is_ok_and(|given| given == port)
}

/// `origin`（请求的 `Origin` 头）是不是 `http://` 加同一个 `host`。没带的（不是浏览器发的）算是。
pub(super) fn same_origin(origin: Option<&str>, host: &str) -> bool {
    origin.is_none_or(|origin| origin.strip_prefix("http://") == Some(host))
}
