use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Instant;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(50)
        .build()?;
    let total_reqs = 5000;
    let concurrency = 50;
    let success_count = Arc::new(AtomicUsize::new(0));

    println!("⚡ Sending {} requests across {} concurrent workers to live RustStack...", total_reqs, concurrency);
    let t0 = Instant::now();

    let mut handles = Vec::new();
    let reqs_per_worker = total_reqs / concurrency;

    for _ in 0..concurrency {
        let client = client.clone();
        let counter = success_count.clone();
        handles.push(tokio::spawn(async move {
            for _ in 0..reqs_per_worker {
                if let Ok(resp) = client.get("http://localhost:4566/_ruststack/health").send().await {
                    if resp.status().is_success() {
                        counter.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        }));
    }

    for h in handles {
        h.await?;
    }

    let elapsed = t0.elapsed();
    let succ = success_count.load(Ordering::Relaxed);
    let throughput = (succ as f64) / elapsed.as_secs_f64();

    println!("✅ Completed {}/{} successful requests in {:.2} ms ({:.0} requests/sec)!", succ, total_reqs, elapsed.as_secs_f64() * 1000.0, throughput);

    Ok(())
}
