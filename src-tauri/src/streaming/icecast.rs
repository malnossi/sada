use crate::config::ServerConfig;
use crate::streaming::Stream;
use base64::Engine as _;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Connect to an Icecast server using the HTTP PUT source protocol.
/// Uses HTTP/1.0 (some Icecast versions reject 1.1).
/// Authorization uses HTTP Basic with username "source".
pub async fn connect_icecast(
    cfg: &ServerConfig,
    content_type: &str,
    bitrate_kbps: u32,
) -> anyhow::Result<Stream> {
    let addr = format!("{}:{}", cfg.host, cfg.port);
    let tcp_stream = TcpStream::connect(&addr)
        .await
        .map_err(|e| anyhow::anyhow!("TCP connect to {addr}: {e}"))?;

    let mut stream = if cfg.tls {
        let connector = native_tls::TlsConnector::new()
            .map_err(|e| anyhow::anyhow!("Failed to create TLS connector: {e}"))?;
        let tokio_connector = tokio_native_tls::TlsConnector::from(connector);
        let tls_stream = tokio_connector
            .connect(&cfg.host, tcp_stream)
            .await
            .map_err(|e| anyhow::anyhow!("TLS handshake with {}: {e}", cfg.host))?;
        Stream::Tls(tls_stream)
    } else {
        Stream::Plain(tcp_stream)
    };

    let credentials =
        base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", cfg.username, cfg.source_password));

    let mount = if cfg.mount.starts_with('/') {
        cfg.mount.clone()
    } else {
        format!("/{}", cfg.mount)
    };
    let public_str = if cfg.public { "1" } else { "0" };

    let request = if cfg.legacy_icecast {
        // Use legacy SOURCE protocol instead of PUT
        format!(
            "SOURCE {mount} ICE/1.0\r\n\
             Authorization: Basic {credentials}\r\n\
             Content-Type: {content_type}\r\n\
             User-Agent: Sada/0.1.0\r\n\
             ice-name: {name}\r\n\
             ice-url: {url}\r\n\
             ice-genre: {genre}\r\n\
             ice-bitrate: {bitrate}\r\n\
             ice-public: {public}\r\n\
             ice-description: {desc}\r\n\
             \r\n",
            mount = mount,
            credentials = credentials,
            content_type = content_type,
            name = escape_header(&cfg.stream_name),
            url = escape_header(&cfg.stream_url),
            genre = escape_header(&cfg.stream_genre),
            bitrate = bitrate_kbps,
            public = public_str,
            desc = escape_header(&cfg.stream_description),
        )
    } else {
        // Modern Icecast v2 PUT protocol
        format!(
            "PUT {mount} HTTP/1.0\r\n\
             Host: {host}:{port}\r\n\
             Authorization: Basic {credentials}\r\n\
             Content-Type: {content_type}\r\n\
             User-Agent: Sada/0.1.0\r\n\
             Ice-Public: {public}\r\n\
             Ice-Name: {name}\r\n\
             Ice-Description: {desc}\r\n\
             Ice-URL: {url}\r\n\
             Ice-Genre: {genre}\r\n\
             Ice-Bitrate: {bitrate}\r\n\
             Expect: 100-continue\r\n\
             \r\n",
            mount = mount,
            host = cfg.host,
            port = cfg.port,
            credentials = credentials,
            content_type = content_type,
            public = public_str,
            name = escape_header(&cfg.stream_name),
            desc = escape_header(&cfg.stream_description),
            url = escape_header(&cfg.stream_url),
            genre = escape_header(&cfg.stream_genre),
            bitrate = bitrate_kbps,
        )
    };

    stream.write_all(request.as_bytes()).await?;
    stream.flush().await?;

    // Read and validate the 100 Continue response
    let mut buf = [0u8; 512];
    let n = stream.read(&mut buf).await?;
    let response = std::str::from_utf8(&buf[..n]).unwrap_or("");

    if !response.contains("100") && !response.contains("200") {
        anyhow::bail!("Icecast rejected connection: {response}");
    }

    Ok(stream)
}

/// Strip newlines and carriage returns from header values to prevent header injection
fn escape_header(s: &str) -> String {
    s.replace(['\r', '\n'], "")
}
