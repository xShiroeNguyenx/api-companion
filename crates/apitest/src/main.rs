//! CLI smoke-test cho http-engine.
//!
//! Dùng: `cargo run -p apitest -- <URL> [METHOD]`
//! Ví dụ: `cargo run -p apitest -- https://example.com`

use ipc_types::{HttpMethod, RequestSpec};

#[tokio::main]
async fn main() {
    let mut args = std::env::args().skip(1);
    let url = args.next().unwrap_or_else(|| {
        eprintln!("Dùng: apitest <URL> [METHOD]");
        std::process::exit(2);
    });
    let method = args.next().unwrap_or_else(|| "GET".into());

    let mut spec = RequestSpec::get(&url);
    spec.method = HttpMethod::new(method);

    println!("→ {} {}", spec.method.as_str(), spec.url);
    let rec = http_engine::send(&spec).await;

    if let Some(err) = &rec.error {
        eprintln!("✗ Lỗi: [{:?}] {}", err.code, err.message);
    }

    let t = &rec.timings;
    println!("\n── Timings (ms) ──");
    print_timing("DNS", t.dns_ms);
    print_timing("TCP connect", t.tcp_connect_ms);
    print_timing("TLS handshake", t.tls_handshake_ms);
    print_timing("TTFB", t.ttfb_ms);
    print_timing("Download", t.download_ms);
    print_timing("TOTAL", t.total_ms);

    if let Some(tls) = &rec.tls {
        println!("\n── TLS ──");
        println!("  version: {:?}", tls.protocol_version);
        println!("  cipher : {:?}", tls.cipher_suite);
        println!("  alpn   : {:?}", tls.alpn);
        if let Some(cert) = tls.peer_certificates.first() {
            println!("  cert   : {} (issuer: {})", cert.subject, cert.issuer);
            println!("           hết hạn: {:?}", cert.not_after);
        }
    }

    if !rec.redirects.is_empty() {
        println!("\n── Redirects ──");
        for hop in &rec.redirects {
            println!("  {} {} → {}", hop.status, hop.from_url, hop.location);
        }
    }

    if let Some(resp) = &rec.response {
        println!("\n── Response ──");
        println!("  {} {} ({})", resp.status, resp.status_text, resp.http_version);
        println!("  remote: {:?}", resp.remote_addr);
        println!("  headers: {} dòng", resp.headers.len());
        println!(
            "  body: {} bytes (raw {} bytes){}",
            resp.body.size,
            resp.body.raw_size,
            resp.body
                .content_encoding
                .as_deref()
                .map(|e| format!(", encoding: {e}"))
                .unwrap_or_default()
        );
        if let Some(text) = &resp.body.text {
            let preview: String = text.chars().take(240).collect();
            println!("\n  preview:\n{preview}");
        }
    }
}

fn print_timing(label: &str, v: Option<f64>) {
    match v {
        Some(ms) => println!("  {label:<14}: {ms:>8.2}"),
        None => println!("  {label:<14}:        -"),
    }
}
