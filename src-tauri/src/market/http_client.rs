use std::net::IpAddr;

use anyhow::{bail, Context, Result};
use reqwest::header::{HeaderMap, HeaderValue, REFERER, USER_AGENT};
use reqwest::{Client, Proxy};

#[derive(Clone, Debug)]
pub struct PolicySettings {
    pub use_local_proxy: bool,
    pub proxy_protocol: String,
    pub proxy_host: String,
    pub proxy_port: u16,
}

impl Default for PolicySettings {
    fn default() -> Self {
        Self {
            use_local_proxy: false,
            proxy_protocol: "socks5".to_string(),
            proxy_host: "127.0.0.1".to_string(),
            proxy_port: 10808,
        }
    }
}

fn proxy_url_for_policy(policy: &PolicySettings) -> Result<Option<String>> {
    if !policy.use_local_proxy {
        return Ok(None);
    }

    let protocol = match policy.proxy_protocol.as_str() {
        "socks5" => "socks5",
        "http" => "http",
        unsupported => bail!("unsupported local proxy protocol: {unsupported}"),
    };
    if policy.proxy_port == 0 {
        bail!("local proxy port must be between 1 and 65535");
    }
    let host: IpAddr = policy
        .proxy_host
        .parse()
        .context("local proxy host must be a valid IP address")?;
    let host = match host {
        IpAddr::V4(address) => address.to_string(),
        IpAddr::V6(address) => format!("[{address}]"),
    };

    Ok(Some(format!("{protocol}://{host}:{}", policy.proxy_port)))
}

fn proxy_for_policy(policy: &PolicySettings) -> Result<Option<Proxy>> {
    proxy_url_for_policy(policy)?
        .map(|proxy_url| Proxy::all(&proxy_url).context("invalid local proxy URL"))
        .transpose()
}

pub fn build_http_client(policy: &PolicySettings) -> Result<Client> {
    let mut default_headers = HeaderMap::new();
    default_headers.insert(USER_AGENT, HeaderValue::from_static("rust-reqwest/0.11"));
    default_headers.insert(
        REFERER,
        HeaderValue::from_static("https://freeserv.dukascopy.com/2.0/"),
    );

    let mut client_builder = Client::builder()
        .no_proxy()
        .default_headers(default_headers);
    if let Some(proxy) = proxy_for_policy(policy)? {
        client_builder = client_builder.proxy(proxy);
    }

    client_builder
        .build()
        .context("failed to build Dukascopy HTTP client")
}

#[cfg(test)]
mod tests {
    use super::{build_http_client, proxy_url_for_policy, PolicySettings};

    #[test]
    fn builds_local_socks5_and_http_proxy_urls() {
        let socks5 = PolicySettings {
            use_local_proxy: true,
            ..PolicySettings::default()
        };
        let http = PolicySettings {
            use_local_proxy: true,
            proxy_protocol: "http".to_string(),
            proxy_port: 10809,
            ..PolicySettings::default()
        };

        assert_eq!(
            proxy_url_for_policy(&socks5).unwrap().as_deref(),
            Some("socks5://127.0.0.1:10808")
        );
        assert_eq!(
            proxy_url_for_policy(&http).unwrap().as_deref(),
            Some("http://127.0.0.1:10809")
        );
        assert!(build_http_client(&socks5).is_ok());
        assert!(build_http_client(&http).is_ok());
        assert!(proxy_url_for_policy(&PolicySettings::default())
            .unwrap()
            .is_none());
        assert!(build_http_client(&PolicySettings::default()).is_ok());
    }
}
