//! # http-engine — Lõi HTTP của API Companion
//!
//! Dựng trên `hyper` ở tầng thấp (client::conn) thay vì reqwest, để đo được
//! timing từng phase (DNS / TCP / TLS / TTFB / download), lấy TLS cert chain,
//! và giữ raw bytes trước decompress. Xem docs/adr/0002-hyper-not-reqwest.md.
//!
//! API chính: [`send`] nhận một [`RequestSpec`] và trả về [`ExchangeRecord`].

use std::io::Read as _;
use std::sync::Arc;
use std::time::{Duration, Instant};

use base64::Engine as _;
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper_util::rt::TokioIo;
use ipc_types::{
    AppError, CertSummary, ErrorCode, ExchangeRecord, KeyValue, RedirectHop, RequestBody,
    RequestSpec, ResponseBody, ResponseRecord, Timings, TlsInfo,
};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio_util::sync::CancellationToken;

const DEFAULT_TIMEOUT_MS: u64 = 30_000;
const USER_AGENT: &str = concat!("APICompanion/", env!("CARGO_PKG_VERSION"));
const MULTIPART_BOUNDARY: &str = "----APICompanionBoundary7MA4YWxkTrZu0gW";

/// Stream hợp nhất TCP (http) và TLS (https) để đưa vào hyper.
trait IoStream: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> IoStream for T {}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

/// Thực thi một request và trả về kết quả đầy đủ.
///
/// Không bao giờ trả `Err` cho lỗi mạng — lỗi được gói vào `ExchangeRecord.error`
/// để UI hiển thị đồng nhất. Chỉ trả `Err` khi input sai ngay từ đầu (URL rỗng...).
pub async fn send(spec: &RequestSpec) -> ExchangeRecord {
    send_with_cancel(spec, &CancellationToken::new()).await
}

/// Như [`send`] nhưng có thể hủy giữa chừng qua `cancel`.
///
/// Khi `cancel` được kích hoạt, mọi I/O đang chờ bị bỏ ngay và trả về
/// `ExchangeRecord` với `ErrorCode::Cancelled`.
pub async fn send_with_cancel(spec: &RequestSpec, cancel: &CancellationToken) -> ExchangeRecord {
    let started = Instant::now();
    let timeout = Duration::from_millis(spec.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS));

    tokio::select! {
        _ = cancel.cancelled() => {
            let mut rec = error_record(
                spec,
                started,
                AppError::new(ErrorCode::Cancelled, "Request đã bị hủy"),
            );
            rec.timings.total_ms = Some(ms(started.elapsed()));
            rec
        }
        res = tokio::time::timeout(timeout, run_with_redirects(spec, started)) => match res {
            Ok(Ok(record)) => record,
            Ok(Err(err)) => error_record(spec, started, err),
            Err(_) => {
                let mut rec = error_record(
                    spec,
                    started,
                    AppError::new(
                        ErrorCode::Timeout,
                        format!("Request quá {}ms", timeout.as_millis()),
                    ),
                );
                rec.timings.total_ms = Some(ms(started.elapsed()));
                rec
            }
        }
    }
}

fn error_record(spec: &RequestSpec, started: Instant, err: AppError) -> ExchangeRecord {
    ExchangeRecord {
        final_url: spec.url.clone(),
        method: spec.method.as_str().to_string(),
        response: None,
        timings: Timings {
            total_ms: Some(ms(started.elapsed())),
            ..Default::default()
        },
        tls: None,
        redirects: Vec::new(),
        error: Some(err),
    }
}

/// Vòng lặp redirect. Mỗi lần lặp là một exchange đơn tới `current_url`.
async fn run_with_redirects(
    spec: &RequestSpec,
    started: Instant,
) -> Result<ExchangeRecord, AppError> {
    let mut current_url = build_url_with_query(&spec.url, &spec.query)?;
    let mut method = spec.method.as_str().to_uppercase();
    // Body chỉ dựng một lần; redirect đổi sang GET sẽ bỏ body.
    let mut body = build_body(&spec.body)?;
    let mut extra_content_type = body.content_type.take();

    let mut hops: Vec<RedirectHop> = Vec::new();

    loop {
        let single = exchange_once(
            &current_url,
            &method,
            &spec.headers,
            &spec.auth,
            &body.bytes,
            extra_content_type.as_deref(),
            spec.verify_tls,
        )
        .await?;

        // Timing/TLS của hop hiện tại (hop cuối là đại diện khi trả về).
        let mut timings = single.timings;
        let tls_info = single.tls;

        let status = single.status;
        let is_redirect = (300..400).contains(&status) && status != 304;

        if spec.follow_redirects && is_redirect {
            if let Some(location) = single
                .headers
                .iter()
                .find(|kv| kv.key.eq_ignore_ascii_case("location"))
                .map(|kv| kv.value.clone())
            {
                if hops.len() as u32 >= spec.max_redirects {
                    return Err(AppError::new(
                        ErrorCode::TooManyRedirects,
                        format!("Vượt quá {} redirect", spec.max_redirects),
                    ));
                }
                let next = resolve_redirect(&current_url, &location)?;
                hops.push(RedirectHop {
                    status,
                    from_url: current_url.clone(),
                    location: next.clone(),
                });
                // 301/302/303 → chuyển POST/PUT... về GET và bỏ body (theo thực tiễn trình duyệt).
                if status == 301 || status == 302 || status == 303 {
                    if method != "GET" && method != "HEAD" {
                        method = "GET".to_string();
                        body.bytes = Bytes::new();
                        extra_content_type = None;
                    }
                }
                current_url = next;
                continue;
            }
        }

        timings.total_ms = Some(ms(started.elapsed()));
        return Ok(ExchangeRecord {
            final_url: current_url,
            method,
            response: Some(ResponseRecord {
                status,
                status_text: reason_phrase(status).to_string(),
                http_version: single.http_version,
                headers: single.headers,
                body: single.body,
                remote_addr: single.remote_addr,
            }),
            timings,
            tls: tls_info,
            redirects: hops,
            error: None,
        });
    }
}

/// Kết quả một exchange đơn (chưa xét redirect).
struct SingleExchange {
    status: u16,
    http_version: String,
    headers: Vec<KeyValue>,
    body: ResponseBody,
    remote_addr: Option<String>,
    timings: Timings,
    tls: Option<TlsInfo>,
}

#[allow(clippy::too_many_arguments)]
async fn exchange_once(
    url_str: &str,
    method: &str,
    user_headers: &[KeyValue],
    auth: &ipc_types::Auth,
    body_bytes: &Bytes,
    extra_content_type: Option<&str>,
    verify_tls: bool,
) -> Result<SingleExchange, AppError> {
    let url = url::Url::parse(url_str)
        .map_err(|e| AppError::new(ErrorCode::InvalidUrl, format!("URL không hợp lệ: {e}")))?;
    let scheme = url.scheme();
    let https = match scheme {
        "http" => false,
        "https" => true,
        other => {
            return Err(AppError::new(
                ErrorCode::Unsupported,
                format!("Scheme chưa hỗ trợ: {other}"),
            ))
        }
    };
    let host = url
        .host_str()
        .ok_or_else(|| AppError::new(ErrorCode::InvalidUrl, "Thiếu host"))?
        .to_string();
    let port = url.port_or_known_default().unwrap_or(if https { 443 } else { 80 });

    let mut timings = Timings::default();

    // --- DNS ---
    let t = Instant::now();
    let addrs: Vec<std::net::SocketAddr> = tokio::net::lookup_host((host.as_str(), port))
        .await
        .map_err(|e| AppError::new(ErrorCode::DnsFailed, format!("DNS lỗi: {e}")))?
        .collect();
    timings.dns_ms = Some(ms(t.elapsed()));
    let addr = addrs
        .into_iter()
        .next()
        .ok_or_else(|| AppError::new(ErrorCode::DnsFailed, "Không phân giải được địa chỉ"))?;

    // --- TCP ---
    let t = Instant::now();
    let tcp = TcpStream::connect(addr)
        .await
        .map_err(|e| AppError::new(ErrorCode::ConnectFailed, format!("TCP lỗi: {e}")))?;
    timings.tcp_connect_ms = Some(ms(t.elapsed()));
    let _ = tcp.set_nodelay(true);
    let remote_addr = tcp.peer_addr().ok().map(|a| a.to_string());

    // --- TLS (nếu https) ---
    let (stream, tls_info): (Box<dyn IoStream>, Option<TlsInfo>) = if https {
        if !verify_tls {
            return Err(AppError::new(
                ErrorCode::Unsupported,
                "Bỏ qua verify TLS chưa được hỗ trợ ở phiên bản này",
            ));
        }
        let config = build_tls_config()?;
        let server_name = rustls_pki_types::ServerName::try_from(host.clone())
            .map_err(|e| AppError::new(ErrorCode::TlsFailed, format!("Server name lỗi: {e}")))?;
        let connector = tokio_rustls::TlsConnector::from(Arc::new(config));
        let t = Instant::now();
        let tls_stream = connector
            .connect(server_name, tcp)
            .await
            .map_err(|e| AppError::new(ErrorCode::TlsFailed, format!("TLS handshake lỗi: {e}")))?;
        timings.tls_handshake_ms = Some(ms(t.elapsed()));
        let info = extract_tls_info(&tls_stream);
        (Box::new(tls_stream), Some(info))
    } else {
        (Box::new(tcp), None)
    };

    // --- HTTP/1.1 handshake + gửi request ---
    let io = TokioIo::new(stream);
    let (mut sender, conn) = hyper::client::conn::http1::Builder::new()
        .preserve_header_case(true)
        .title_case_headers(true)
        .handshake::<_, Full<Bytes>>(io)
        .await
        .map_err(|e| AppError::new(ErrorCode::RequestFailed, format!("Handshake lỗi: {e}")))?;
    let conn_task = tokio::spawn(async move {
        let _ = conn.await;
    });

    let request = build_http_request(&url, method, &host, port, https, user_headers, auth, body_bytes, extra_content_type)?;

    let t_send = Instant::now();
    let resp = sender
        .send_request(request)
        .await
        .map_err(|e| AppError::new(ErrorCode::RequestFailed, format!("Gửi request lỗi: {e}")))?;
    timings.ttfb_ms = Some(ms(t_send.elapsed()));

    let status = resp.status().as_u16();
    let http_version = format!("{:?}", resp.version());
    let headers: Vec<KeyValue> = resp
        .headers()
        .iter()
        .map(|(k, v)| KeyValue {
            key: k.as_str().to_string(),
            value: String::from_utf8_lossy(v.as_bytes()).to_string(),
            enabled: true,
        })
        .collect();
    let content_encoding = resp
        .headers()
        .get(http::header::CONTENT_ENCODING)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    // --- Đọc body ---
    let t_dl = Instant::now();
    let raw = resp
        .into_body()
        .collect()
        .await
        .map_err(|e| AppError::new(ErrorCode::BodyReadFailed, format!("Đọc body lỗi: {e}")))?
        .to_bytes();
    timings.download_ms = Some(ms(t_dl.elapsed()));

    conn_task.abort();

    let body = build_response_body(&raw, content_encoding.as_deref());

    Ok(SingleExchange {
        status,
        http_version,
        headers,
        body,
        remote_addr,
        timings,
        tls: tls_info,
    })
}

// ---------------------------------------------------------------------------
// TLS
// ---------------------------------------------------------------------------

fn build_tls_config() -> Result<rustls::ClientConfig, AppError> {
    let mut roots = rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let mut config = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|e| AppError::new(ErrorCode::TlsFailed, format!("Cấu hình TLS lỗi: {e}")))?
        .with_root_certificates(roots)
        .with_no_client_auth();
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(config)
}

fn extract_tls_info<T>(stream: &tokio_rustls::client::TlsStream<T>) -> TlsInfo {
    let (_, conn) = stream.get_ref();
    let protocol_version = conn.protocol_version().map(|v| format!("{v:?}"));
    let cipher_suite = conn.negotiated_cipher_suite().map(|c| format!("{:?}", c.suite()));
    let alpn = conn
        .alpn_protocol()
        .map(|p| String::from_utf8_lossy(p).to_string());
    let peer_certificates = conn
        .peer_certificates()
        .map(|certs| certs.iter().map(summarize_cert).collect())
        .unwrap_or_default();
    TlsInfo {
        protocol_version,
        cipher_suite,
        alpn,
        peer_certificates,
    }
}

fn summarize_cert(der: &rustls_pki_types::CertificateDer) -> CertSummary {
    match x509_parser::parse_x509_certificate(der.as_ref()) {
        Ok((_, cert)) => CertSummary {
            subject: cert.subject().to_string(),
            issuer: cert.issuer().to_string(),
            not_before: Some(cert.validity().not_before.to_string()),
            not_after: Some(cert.validity().not_after.to_string()),
            serial: Some(cert.raw_serial_as_string()),
        },
        Err(_) => CertSummary {
            subject: "<parse failed>".into(),
            issuer: String::new(),
            not_before: None,
            not_after: None,
            serial: None,
        },
    }
}

// ---------------------------------------------------------------------------
// Dựng request
// ---------------------------------------------------------------------------

/// Body đã render sẵn bytes + content-type mặc định (nếu có).
struct BuiltBody {
    bytes: Bytes,
    content_type: Option<String>,
}

fn build_body(body: &RequestBody) -> Result<BuiltBody, AppError> {
    match body {
        RequestBody::None => Ok(BuiltBody {
            bytes: Bytes::new(),
            content_type: None,
        }),
        RequestBody::Text { content, content_type } => Ok(BuiltBody {
            bytes: Bytes::from(content.clone().into_bytes()),
            content_type: content_type.clone(),
        }),
        RequestBody::Form { fields } => {
            let mut ser = form_urlencoded::Serializer::new(String::new());
            for f in fields.iter().filter(|f| f.enabled && !f.key.trim().is_empty()) {
                ser.append_pair(&f.key, &f.value);
            }
            Ok(BuiltBody {
                bytes: Bytes::from(ser.finish().into_bytes()),
                content_type: Some("application/x-www-form-urlencoded".into()),
            })
        }
        RequestBody::Multipart { parts } => build_multipart(parts),
        RequestBody::BinaryFile { path, content_type } => {
            let data = std::fs::read(path)
                .map_err(|e| AppError::new(ErrorCode::Io, format!("Đọc file '{path}' lỗi: {e}")))?;
            Ok(BuiltBody {
                bytes: Bytes::from(data),
                content_type: content_type.clone(),
            })
        }
    }
}

fn build_multipart(parts: &[ipc_types::MultipartPart]) -> Result<BuiltBody, AppError> {
    let mut buf: Vec<u8> = Vec::new();
    for part in parts.iter().filter(|p| p.enabled) {
        buf.extend_from_slice(format!("--{MULTIPART_BOUNDARY}\r\n").as_bytes());
        match &part.file_path {
            Some(path) => {
                let filename = std::path::Path::new(path)
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| "file".into());
                buf.extend_from_slice(
                    format!(
                        "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\n",
                        part.name, filename
                    )
                    .as_bytes(),
                );
                let ct = part.content_type.clone().unwrap_or_else(|| "application/octet-stream".into());
                buf.extend_from_slice(format!("Content-Type: {ct}\r\n\r\n").as_bytes());
                let data = std::fs::read(path).map_err(|e| {
                    AppError::new(ErrorCode::Io, format!("Đọc file '{path}' lỗi: {e}"))
                })?;
                buf.extend_from_slice(&data);
                buf.extend_from_slice(b"\r\n");
            }
            None => {
                buf.extend_from_slice(
                    format!("Content-Disposition: form-data; name=\"{}\"\r\n\r\n", part.name)
                        .as_bytes(),
                );
                buf.extend_from_slice(part.value.as_bytes());
                buf.extend_from_slice(b"\r\n");
            }
        }
    }
    buf.extend_from_slice(format!("--{MULTIPART_BOUNDARY}--\r\n").as_bytes());
    Ok(BuiltBody {
        bytes: Bytes::from(buf),
        content_type: Some(format!("multipart/form-data; boundary={MULTIPART_BOUNDARY}")),
    })
}

#[allow(clippy::too_many_arguments)]
fn build_http_request(
    url: &url::Url,
    method: &str,
    host: &str,
    port: u16,
    https: bool,
    user_headers: &[KeyValue],
    auth: &ipc_types::Auth,
    body_bytes: &Bytes,
    extra_content_type: Option<&str>,
) -> Result<http::Request<Full<Bytes>>, AppError> {
    // Request target = origin-form (path?query).
    let mut path = url.path().to_string();
    if path.is_empty() {
        path = "/".into();
    }
    if let Some(q) = url.query() {
        path.push('?');
        path.push_str(q);
    }

    let method = http::Method::from_bytes(method.as_bytes())
        .map_err(|e| AppError::new(ErrorCode::RequestFailed, format!("Method lỗi: {e}")))?;

    let mut builder = http::Request::builder().method(method).uri(&path);

    // Host header (kèm port nếu không phải mặc định).
    let default_port = if https { 443 } else { 80 };
    let host_header = if port == default_port {
        host.to_string()
    } else {
        format!("{host}:{port}")
    };
    builder = builder
        .header(http::header::HOST, host_header)
        .header(http::header::USER_AGENT, USER_AGENT)
        .header(http::header::ACCEPT, "*/*")
        .header(http::header::ACCEPT_ENCODING, "gzip, deflate, br");

    // Content-Type mặc định từ body (nếu user chưa tự set).
    let user_has_ct = user_headers
        .iter()
        .any(|h| h.enabled && h.key.eq_ignore_ascii_case("content-type"));
    if !user_has_ct {
        if let Some(ct) = extra_content_type {
            builder = builder.header(http::header::CONTENT_TYPE, ct);
        }
    }

    // Auth.
    match auth {
        ipc_types::Auth::Bearer { token } => {
            builder = builder.header(http::header::AUTHORIZATION, format!("Bearer {token}"));
        }
        ipc_types::Auth::Basic { username, password } => {
            let raw = format!("{username}:{password}");
            let encoded = base64::engine::general_purpose::STANDARD.encode(raw);
            builder = builder.header(http::header::AUTHORIZATION, format!("Basic {encoded}"));
        }
        ipc_types::Auth::ApiKey {
            key,
            value,
            location,
        } => {
            if matches!(location, ipc_types::ApiKeyLocation::Header) {
                builder = builder.header(key.as_str(), value.as_str());
            }
            // ApiKey ở query đã được nối vào URL ở build_url_with_query nếu cần (v1: header only path).
        }
        _ => {}
    }

    // User headers (ghi đè/bổ sung). Bỏ qua dòng key rỗng (vd. dòng trống cuối bảng)
    // để không sinh header "" -> reqwest báo "invalid HTTP header name".
    for h in user_headers.iter().filter(|h| h.enabled && !h.key.trim().is_empty()) {
        builder = builder.header(h.key.as_str(), h.value.as_str());
    }

    builder
        .body(Full::new(body_bytes.clone()))
        .map_err(|e| AppError::new(ErrorCode::RequestFailed, format!("Dựng request lỗi: {e}")))
}

// ---------------------------------------------------------------------------
// URL & redirect helpers
// ---------------------------------------------------------------------------

fn build_url_with_query(base: &str, query: &[KeyValue]) -> Result<String, AppError> {
    let mut url = url::Url::parse(base)
        .map_err(|e| AppError::new(ErrorCode::InvalidUrl, format!("URL không hợp lệ: {e}")))?;
    let enabled: Vec<&KeyValue> = query.iter().filter(|q| q.enabled && !q.key.trim().is_empty()).collect();
    if !enabled.is_empty() {
        for q in enabled {
            url.query_pairs_mut().append_pair(&q.key, &q.value);
        }
    }
    Ok(url.to_string())
}

fn resolve_redirect(current: &str, location: &str) -> Result<String, AppError> {
    let base = url::Url::parse(current)
        .map_err(|e| AppError::new(ErrorCode::InvalidUrl, format!("URL nền lỗi: {e}")))?;
    let resolved = base
        .join(location)
        .map_err(|e| AppError::new(ErrorCode::InvalidUrl, format!("Location lỗi: {e}")))?;
    Ok(resolved.to_string())
}

// ---------------------------------------------------------------------------
// Response body & decompression
// ---------------------------------------------------------------------------

fn build_response_body(raw: &Bytes, content_encoding: Option<&str>) -> ResponseBody {
    let raw_size = raw.len() as u64;
    let (decoded, enc_label): (Vec<u8>, Option<String>) = match content_encoding {
        Some(enc) if enc.eq_ignore_ascii_case("gzip") => {
            (decompress_gzip(raw).unwrap_or_else(|| raw.to_vec()), Some("gzip".into()))
        }
        Some(enc) if enc.eq_ignore_ascii_case("deflate") => {
            (decompress_deflate(raw).unwrap_or_else(|| raw.to_vec()), Some("deflate".into()))
        }
        Some(enc) if enc.eq_ignore_ascii_case("br") => {
            (decompress_brotli(raw).unwrap_or_else(|| raw.to_vec()), Some("br".into()))
        }
        Some(other) => (raw.to_vec(), Some(other.to_string())),
        None => (raw.to_vec(), None),
    };

    let size = decoded.len() as u64;
    match String::from_utf8(decoded) {
        Ok(text) => ResponseBody {
            text: Some(text),
            base64: None,
            size,
            raw_size,
            content_encoding: enc_label,
        },
        Err(e) => ResponseBody {
            text: None,
            base64: Some(base64::engine::general_purpose::STANDARD.encode(e.into_bytes())),
            size,
            raw_size,
            content_encoding: enc_label,
        },
    }
}

fn decompress_gzip(data: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    flate2::read::GzDecoder::new(data).read_to_end(&mut out).ok()?;
    Some(out)
}

fn decompress_deflate(data: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    // Thử zlib-wrapped trước, fallback raw deflate.
    if flate2::read::ZlibDecoder::new(data).read_to_end(&mut out).is_ok() {
        return Some(out);
    }
    out.clear();
    flate2::read::DeflateDecoder::new(data)
        .read_to_end(&mut out)
        .ok()?;
    Some(out)
}

fn decompress_brotli(data: &[u8]) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    brotli::Decompressor::new(data, 4096)
        .read_to_end(&mut out)
        .ok()?;
    Some(out)
}

// ---------------------------------------------------------------------------
// Reason phrase
// ---------------------------------------------------------------------------

fn reason_phrase(status: u16) -> &'static str {
    http::StatusCode::from_u16(status)
        .ok()
        .and_then(|s| s.canonical_reason())
        .unwrap_or("")
}

// `form_urlencoded` đi kèm crate `url`.
use url::form_urlencoded;

#[cfg(test)]
mod tests {
    use super::*;
    use ipc_types::{HttpMethod, RequestSpec};

    #[test]
    fn builds_url_with_query() {
        let q = vec![
            KeyValue { key: "a".into(), value: "1".into(), enabled: true },
            KeyValue { key: "b".into(), value: "2".into(), enabled: false },
        ];
        let out = build_url_with_query("https://x.com/p", &q).unwrap();
        assert!(out.contains("a=1"));
        assert!(!out.contains("b=2"));
    }

    #[test]
    fn skips_empty_key_headers() {
        // Dòng trống cuối bảng (key="") không được sinh thành header rỗng -> tránh "invalid HTTP header name".
        let url = url::Url::parse("http://g-api.test/x").unwrap();
        let headers = vec![
            KeyValue { key: "Content-Type".into(), value: "application/json".into(), enabled: true },
            KeyValue { key: "".into(), value: "".into(), enabled: true },
            KeyValue { key: "  ".into(), value: "bỏ".into(), enabled: true },
        ];
        let req = build_http_request(
            &url,
            "POST",
            "g-api.test",
            80,
            false,
            &headers,
            &ipc_types::Auth::None,
            &Bytes::new(),
            None,
        )
        .expect("build không được lỗi vì header rỗng");
        assert_eq!(req.headers().get("content-type").unwrap(), "application/json");
        // Không có header nào có tên rỗng/whitespace.
        assert!(req.headers().keys().all(|k| !k.as_str().trim().is_empty()));
    }

    #[test]
    fn resolves_relative_redirect() {
        let out = resolve_redirect("https://x.com/a/b", "/c").unwrap();
        assert_eq!(out, "https://x.com/c");
    }

    #[test]
    fn gzip_roundtrip() {
        use flate2::write::GzEncoder;
        use std::io::Write;
        let mut enc = GzEncoder::new(Vec::new(), flate2::Compression::default());
        enc.write_all(b"hello world").unwrap();
        let compressed = enc.finish().unwrap();
        let body = build_response_body(&Bytes::from(compressed), Some("gzip"));
        assert_eq!(body.text.as_deref(), Some("hello world"));
        assert_eq!(body.content_encoding.as_deref(), Some("gzip"));
    }

    #[tokio::test]
    async fn empty_url_is_error() {
        let spec = RequestSpec {
            method: HttpMethod::get(),
            url: "".into(),
            ..RequestSpec::get("")
        };
        let rec = send(&spec).await;
        assert!(rec.error.is_some());
    }

    #[tokio::test]
    async fn pre_cancelled_token_returns_cancelled() {
        let token = CancellationToken::new();
        token.cancel(); // hủy trước khi chạy
        let spec = RequestSpec::get("https://example.com");
        let rec = send_with_cancel(&spec, &token).await;
        assert!(matches!(
            rec.error.as_ref().map(|e| e.code),
            Some(ipc_types::ErrorCode::Cancelled)
        ));
        assert!(rec.response.is_none());
    }
}
