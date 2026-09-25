//! Security module providing Server-Side Request Forgery (SSRF) validation.
//!
//! This module parses candidate URLs, enforces HTTP/HTTPS protocols, rejects localhost domains,
//! resolves DNS hostnames to target IP addresses, and blocks non-public IPv4 and IPv6 ranges
//! (loopback, private subnets, link-local, broadcast, unspecified, and CGNAT/Tailscale ranges).

use std::net::{IpAddr, ToSocketAddrs};
use url::Url;

/// Validates that a target URL is public and safe to request (SSRF Guard).
///
/// Ensures the URL uses the `http` or `https` scheme, rejects `localhost` domains,
/// resolves the hostname to socket addresses, and checks that none of the resolved IPs
/// belong to private, local, or restricted network ranges.
///
/// # Arguments
/// * `raw_url` - The candidate URL string to validate.
///
/// # Returns
/// * `Ok(())` - If the URL is valid, uses HTTP/HTTPS, and points to a public IP.
/// * `Err(String)` - An English error message describing why the URL was blocked or failed validation.
pub fn validate_public_url(raw_url: &str) -> Result<(), String> {
    let parsed = Url::parse(raw_url).map_err(|e| format!("Invalid URL: {}", e))?;

    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Err("Only HTTP and HTTPS protocols are allowed.".to_string());
    }

    let host = parsed.host_str().ok_or("URL missing host component")?;
    if host.eq_ignore_ascii_case("localhost") || host.ends_with(".localhost") {
        return Err("Access to localhost is blocked (SSRF Guard).".to_string());
    }

    let port = parsed.port_or_known_default().unwrap_or(80);
    let socket_str = format!("{}:{}", host, port);

    let addrs = socket_str
        .to_socket_addrs()
        .map_err(|e| format!("Failed to resolve DNS host: {}", e))?;

    for addr in addrs {
        let ip = addr.ip();
        if is_non_public_ip(&ip) {
            return Err(format!("Access to private/local IP ({}) is blocked (SSRF Guard).", ip));
        }
    }

    Ok(())
}

/// Checks whether an IP address belongs to a non-public or restricted network range.
///
/// # Checks Executed:
/// * **IPv4:** Loopback (`127.0.0.0/8`), Private (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`),
///   Link-Local (`169.254.0.0/16`), Broadcast, Unspecified (`0.0.0.0`), and CGNAT/Tailscale (`100.64.0.0/10`).
/// * **IPv6:** Loopback (`::1`) and Unspecified (`::`).
fn is_non_public_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(ipv4) => {
            ipv4.is_loopback()
                || ipv4.is_private()
                || ipv4.is_link_local()
                || ipv4.is_broadcast()
                || ipv4.is_unspecified()
                // CGNAT Range (Tailscale / Carrier-Grade NAT 100.64.0.0/10)
                || (ipv4.octets()[0] == 100 && (ipv4.octets()[1] & 0xC0) == 64)
        }
        IpAddr::V6(ipv6) => ipv6.is_loopback() || ipv6.is_unspecified(),
    }
}
