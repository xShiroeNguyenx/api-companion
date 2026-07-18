# ADR 0002 — HTTP engine dựng trên hyper, KHÔNG dùng reqwest

**Trạng thái:** Accepted · **Ngày:** 2026-07-13

## Bối cảnh
Là một Postman-replacement, timing waterfall (DNS/TCP/TLS/TTFB/download) và TLS cert chain là **tính năng lõi**, không phải nice-to-have.

## Quyết định
Dựng engine ở tầng thấp bằng `hyper` (client::conn::http1) + `tokio-rustls` + resolver thủ công, thay vì `reqwest`.

## Lý do
- reqwest che mất connection internals: không tách được DNS/TCP/TLS timing, không expose peer certificate chain, khó kiểm soát decompression (cần giữ raw bytes).
- hyper cho phép đo từng phase bằng cách tự điều phối resolve → connect → TLS → handshake.

## Hệ quả (đã hiện thực ở M0)
- `crates/http-engine` tự resolve DNS (`tokio::net::lookup_host`), TCP connect, TLS handshake (rustls **ring** provider — tránh aws-lc-rs cần NASM trên Windows), rồi `hyper::client::conn::http1`.
- Trả về `ExchangeRecord` với `Timings`, `TlsInfo` (version/cipher/alpn/cert), raw + decompressed body.
- v1: HTTP/1.1 + gzip/deflate/br. HTTP/2, HTTP/3 (quinn+h3), insecure-TLS mode: thêm sau qua lớp trait.
