use crate::config::ServerConfig;
use crate::streaming::Stream;
use base64::Engine as _;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Connect to a Shoutcast server using the ICY source protocol.
/// Shoutcast Basic auth uses an empty username: base64(":" + password).
pub async fn connect_shoutcast(
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

    // Shoutcast auth: empty username
    let credentials =
        base64::engine::general_purpose::STANDARD.encode(format!(":{}", cfg.source_password));

    let mount = if cfg.mount.starts_with('/') {
        cfg.mount.clone()
    } else {
        format!("/{}", cfg.mount)
    };

    let request = format!(
        "SOURCE {mount} ICY/1.0\r\n\
         Authorization: Basic {credentials}\r\n\
         Content-Type: {content_type}\r\n\
         icy-name: {name}\r\n\
         icy-description: {desc}\r\n\
         icy-url: {url}\r\n\
         icy-genre: {genre}\r\n\
         icy-bitrate: {bitrate}\r\n\
         icy-pub: {public}\r\n\
         \r\n",
        mount = mount,
        credentials = credentials,
        content_type = content_type,
        name = escape_header(&cfg.stream_name),
        desc = escape_header(&cfg.stream_description),
        url = escape_header(&cfg.stream_url),
        genre = escape_header(&cfg.stream_genre),
        bitrate = bitrate_kbps,
        public = if cfg.public { "1" } else { "0" },
    );

    stream.write_all(request.as_bytes()).await?;
    stream.flush().await?;

    let mut buf = [0u8; 512];
    let n = stream.read(&mut buf).await?;
    let response = std::str::from_utf8(&buf[..n]).unwrap_or("");

    if !response.contains("ICY 200") && !response.contains("200 OK") {
        anyhow::bail!("Shoutcast rejected connection: {response}");
    }

    Ok(stream)
}

/// Strip newlines and carriage returns from header values to prevent header injection
fn escape_header(s: &str) -> String {
    s.replace(['\r', '\n'], "")
}
