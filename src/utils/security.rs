use std::net::{IpAddr, ToSocketAddrs};
use url::Url;

pub fn validate_public_url(raw_url: &str) -> Result<(), String> {
    let parsed = Url::parse(raw_url).map_err(|e| format!("URL tidak valid: {}", e))?;

    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err("Hanya protokol HTTP dan HTTPS yang diizinkan.".to_string());
    }

    let host = parsed.host_str().ok_or("URL tidak memiliki host")?;
    if host.eq_ignore_ascii_case("localhost") || host.ends_with(".localhost") {
        return Err("Akses ke localhost diblokir (SSRF Guard).".to_string());
    }

    let port = parsed.port_or_known_default().unwrap_or(80);
    let socket_str = format!("{}:{}", host, port);

    let addrs = socket_str
        .to_socket_addrs()
        .map_err(|e| format!("Gagal me-resolve host DNS: {}", e))?;

    for addr in addrs {
        let ip = addr.ip();
        if is_non_public_ip(&ip) {
            return Err(format!("Akses ke IP privat/lokal ({}) diblokir (SSRF Guard).", ip));
        }
    }

    Ok(())
}

fn is_non_public_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => {
            ipv4.is_loopback()
                || ipv4.is_private()
                || ipv4.is_link_local()
                || ipv4.is_broadcast()
                || ipv4.is_unspecified()
                // Range CGNAT (Tailscale / Carrier-Grade NAT 100.64.0.0/10)
                || (ipv4.octets()[0] == 100 && (ipv4.octets()[1] & 0xC0) == 64)
        }
        IpAddr::V6(ipv6) => ipv6.is_loopback() || ipv6.is_unspecified(),
    }
}
