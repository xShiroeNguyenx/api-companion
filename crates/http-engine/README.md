# crate: http-engine

**Lõi HTTP.** Thực thi `RequestSpec` → `ExchangeRecord`, đo timing từng phase.

## Public API
```rust
pub async fn send(spec: &ipc_types::RequestSpec) -> ipc_types::ExchangeRecord;
```
Không bao giờ trả `Err` cho lỗi mạng — lỗi gói vào `ExchangeRecord.error`.

## Đã hỗ trợ (M0)
- HTTP/1.1, http:// và https:// (TLS 1.2/1.3 qua rustls **ring**).
- Timing: DNS, TCP connect, TLS handshake, TTFB, download, total.
- TLS info: version, cipher, ALPN, peer cert chain (subject/issuer/validity).
- Body: text/json thô, form-urlencoded, multipart (text + file), binary file.
- Auth: Bearer, Basic, ApiKey (header).
- Redirect following (301/302/303 → GET, ghi hop chain).
- Decompress: gzip, deflate, br. Raw bytes được giữ để tính tỉ lệ nén.

## Chưa (roadmap)
HTTP/2, HTTP/3, insecure-TLS mode, streaming download progress, proxy, cancellation token.

## Phụ thuộc
`ipc-types` + hyper/tokio/rustls stack. Xem docs/adr/0002.
