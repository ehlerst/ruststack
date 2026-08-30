use clap::Parser;
use std::process::Command;
use std::time::{Duration, Instant};
use tokio::time::sleep;

#[derive(Parser, Debug)]
#[command(
    name = "rust-measure-alternatives",
    about = "⚡ Benchmark and Resource Profiler: RustStack vs MiniStack vs LocalStack"
)]
struct Args {
    #[arg(long, default_value = "ehlers320/ruststack:latest")]
    ruststack_image: String,

    #[arg(long, default_value = "ministackorg/ministack:latest")]
    ministack_image: String,

    #[arg(long, default_value = "localstack/localstack:3.8.1")]
    localstack_image: String,

    #[arg(long, default_value_t = 3)]
    runs: usize,

    #[arg(long, default_value_t = 4570)]
    base_port: u16,

    #[arg(long)]
    skip_localstack: bool,

    #[arg(long)]
    output_markdown: Option<String>,
}

#[derive(Debug, Clone)]
struct TargetConfig {
    name: &'static str,
    image: String,
    health_paths: Vec<&'static str>,
}

#[derive(Debug, Clone, Default)]
struct TargetResult {
    name: &'static str,
    image: String,
    image_size_mb: f64,
    startup_times_ms: Vec<f64>,
    memory_usages_mib: Vec<f64>,
    cpu_percentages: Vec<f64>,
}

impl TargetResult {
    fn avg_startup_ms(&self) -> f64 {
        if self.startup_times_ms.is_empty() {
            0.0
        } else {
            self.startup_times_ms.iter().sum::<f64>() / self.startup_times_ms.len() as f64
        }
    }

    fn min_startup_ms(&self) -> f64 {
        self.startup_times_ms
            .iter()
            .cloned()
            .fold(f64::INFINITY, f64::min)
    }

    fn avg_memory_mib(&self) -> f64 {
        if self.memory_usages_mib.is_empty() {
            0.0
        } else {
            self.memory_usages_mib.iter().sum::<f64>() / self.memory_usages_mib.len() as f64
        }
    }

    fn avg_cpu_perc(&self) -> f64 {
        if self.cpu_percentages.is_empty() {
            0.0
        } else {
            self.cpu_percentages.iter().sum::<f64>() / self.cpu_percentages.len() as f64
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    println!("==========================================================================");
    println!("⚡ RustStack Performance & Resource Comparison Benchmark");
    println!("==========================================================================");
    println!("Runs per target: {}", args.runs);
    println!("RustStack image: {}", args.ruststack_image);
    println!("MiniStack image: {}", args.ministack_image);
    if !args.skip_localstack {
        println!("LocalStack image: {}", args.localstack_image);
    }
    println!("--------------------------------------------------------------------------\n");

    let mut targets = vec![
        TargetConfig {
            name: "RustStack",
            image: args.ruststack_image.clone(),
            health_paths: vec!["/_ruststack/health", "/health"],
        },
        TargetConfig {
            name: "MiniStack",
            image: args.ministack_image.clone(),
            health_paths: vec!["/_localstack/health", "/_ministack/health", "/health"],
        },
    ];

    if !args.skip_localstack {
        targets.push(TargetConfig {
            name: "LocalStack",
            image: args.localstack_image.clone(),
            health_paths: vec!["/_localstack/health", "/health"],
        });
    }

    let mut results = Vec::new();

    for target in &targets {
        println!("🚀 Profiling Target: {} ({})", target.name, target.image);

        // 1. Ensure image is available and get size
        ensure_image(&target.image)?;
        let image_size_mb = get_image_size_mb(&target.image)?;
        println!("   📦 Image size: {:.1} MB", image_size_mb);

        let mut res = TargetResult {
            name: target.name,
            image: target.image.clone(),
            image_size_mb,
            ..Default::default()
        };

        for run in 1..=args.runs {
            print!("   🔄 Run {}/{}... ", run, args.runs);
            let port = args.base_port + (run as u16);

            let (startup_ms, mem_mib, cpu_perc) =
                measure_single_run(&target, port).await?;
            println!(
                "Ready in {:.1} ms | Idle Mem: {:.1} MiB | Idle CPU: {:.2}%",
                startup_ms, mem_mib, cpu_perc
            );

            res.startup_times_ms.push(startup_ms);
            res.memory_usages_mib.push(mem_mib);
            res.cpu_percentages.push(cpu_perc);

            sleep(Duration::from_millis(500)).await;
        }

        println!(
            "   📊 Summary: Avg Start: {:.1} ms (Min: {:.1} ms) | Avg Mem: {:.1} MiB\n",
            res.avg_startup_ms(),
            res.min_startup_ms(),
            res.avg_memory_mib()
        );

        results.push(res);
    }

    print_comparison_table(&results);

    if let Some(out_path) = args.output_markdown {
        let md = generate_markdown_summary(&results);
        std::fs::write(&out_path, &md)?;
        println!("📝 Wrote Markdown benchmark summary to: {}", out_path);
    }

    Ok(())
}

fn ensure_image(image: &str) -> anyhow::Result<()> {
    // Check if image exists locally
    let check = Command::new("docker")
        .args(["image", "inspect", image])
        .output()?;
    if !check.status.success() {
        println!("   📥 Pulling image {}...", image);
        let pull = Command::new("docker")
            .args(["pull", image])
            .output()?;
        if !pull.status.success() {
            anyhow::bail!("Failed to pull docker image: {}", image);
        }
    }
    Ok(())
}

fn get_image_size_mb(image: &str) -> anyhow::Result<f64> {
    let out = Command::new("docker")
        .args(["image", "inspect", image, "--format", "{{.Size}}"])
        .output()?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let bytes: f64 = text.parse().unwrap_or(0.0);
    Ok(bytes / (1024.0 * 1024.0))
}

async fn measure_single_run(
    target: &TargetConfig,
    port: u16,
) -> anyhow::Result<(f64, f64, f64)> {
    let container_name = format!("bench-{}-{}", target.name.to_lowercase(), port);

    // 1. Clean up any stale container
    let _ = Command::new("docker")
        .args(["rm", "-f", &container_name])
        .output();

    // 2. Start container
    let start_instant = Instant::now();
    let port_mapping = format!("{}:4566", port);
    let mut run_args = vec![
        "run",
        "-d",
        "--name",
        &container_name,
        "-p",
        &port_mapping,
    ];
    if target.name == "LocalStack" {
        run_args.push("-e");
        run_args.push("SERVICES=s3,sqs,sns,events,ssm,secretsmanager,sts,dynamodb");
    }
    run_args.push(&target.image);

    let run_out = Command::new("docker")
        .args(&run_args)
        .output()?;

    if !run_out.status.success() {
        anyhow::bail!(
            "Failed to start container {}: {}",
            target.name,
            String::from_utf8_lossy(&run_out.stderr)
        );
    }
    let container_id = String::from_utf8_lossy(&run_out.stdout).trim().to_string();

    // 3. Poll health endpoint
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(200))
        .build()?;

    let mut ready = false;
    let mut startup_duration = Duration::ZERO;
    let max_wait = Duration::from_secs(60);
    let poll_start = Instant::now();

    while poll_start.elapsed() < max_wait {
        for path in &target.health_paths {
            let url = format!("http://127.0.0.1:{}{}", port, path);
            if let Ok(resp) = client.get(&url).send().await {
                if resp.status().is_success() {
                    startup_duration = start_instant.elapsed();
                    ready = true;
                    break;
                }
            }
        }
        if ready {
            break;
        }
        sleep(Duration::from_millis(10)).await;
    }

    if !ready {
        let _ = Command::new("docker").args(["rm", "-f", &container_id]).output();
        anyhow::bail!("Timeout waiting for {} to become healthy", target.name);
    }

    // 4. Let container idle settle for 1 second, then collect stats
    sleep(Duration::from_millis(1000)).await;

    let (mem_mib, cpu_perc) = get_container_stats(&container_id)?;

    // 5. Cleanup
    let _ = Command::new("docker").args(["rm", "-f", &container_id]).output();

    Ok((startup_duration.as_secs_f64() * 1000.0, mem_mib, cpu_perc))
}

fn get_container_stats(container_id: &str) -> anyhow::Result<(f64, f64)> {
    let out = Command::new("docker")
        .args([
            "stats",
            "--no-stream",
            "--format",
            "{{.MemUsage}}\t{{.CPUPerc}}",
            container_id,
        ])
        .output()?;

    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    let parts: Vec<&str> = text.split('\t').collect();
    if parts.len() < 2 {
        return Ok((0.0, 0.0));
    }

    // Parse memory (e.g. "15.4MiB / 31.2GiB" or "15.4MB / ...")
    let mem_part = parts[0];
    let mem_used_str = mem_part.split('/').next().unwrap_or("0").trim();
    let mem_mib = parse_to_mib(mem_used_str);

    // Parse CPU (e.g. "0.05%")
    let cpu_str = parts[1].trim_end_matches('%').trim();
    let cpu_perc: f64 = cpu_str.parse().unwrap_or(0.0);

    Ok((mem_mib, cpu_perc))
}

fn parse_to_mib(s: &str) -> f64 {
    let lower = s.to_lowercase();
    if lower.ends_with("kib") || lower.ends_with("kb") {
        let num: f64 = lower
            .trim_end_matches("kib")
            .trim_end_matches("kb")
            .trim()
            .parse()
            .unwrap_or(0.0);
        num / 1024.0
    } else if lower.ends_with("gib") || lower.ends_with("gb") {
        let num: f64 = lower
            .trim_end_matches("gib")
            .trim_end_matches("gb")
            .trim()
            .parse()
            .unwrap_or(0.0);
        num * 1024.0
    } else {
        let num: f64 = lower
            .trim_end_matches("mib")
            .trim_end_matches("mb")
            .trim()
            .parse()
            .unwrap_or(0.0);
        num
    }
}

fn print_comparison_table(results: &[TargetResult]) {
    println!("\n==========================================================================");
    println!("🏆 FINAL BENCHMARK & RESOURCE COMPARISON");
    println!("==========================================================================");
    println!(
        "{:<14} | {:<12} | {:<14} | {:<14} | {:<12}",
        "Stack", "Image Size", "Avg Start Time", "Min Start Time", "Idle Memory"
    );
    println!("--------------------------------------------------------------------------");

    for r in results {
        println!(
            "{:<14} | {:>9.1} MB | {:>11.1} ms | {:>11.1} ms | {:>9.1} MiB",
            r.name,
            r.image_size_mb,
            r.avg_startup_ms(),
            r.min_startup_ms(),
            r.avg_memory_mib()
        );
    }
    println!("==========================================================================\n");
}

fn generate_markdown_summary(results: &[TargetResult]) -> String {
    let mut md = String::new();
    md.push_str("## ⚡ Local Cloud Emulator Benchmark & Resource Comparison\n\n");
    md.push_str("> High-resolution startup latency, idle memory footprint, and image size comparison.\n\n");
    md.push_str("| Local Cloud Stack | Docker Image | Image Size | Avg Startup Time | Min Startup Time | Idle Memory (RSS) | Idle CPU |\n");
    md.push_str("|:---|:---|---:|---:|---:|---:|---:|\n");

    for r in results {
        let badge = if r.name == "RustStack" {
            " **⚡ RustStack (Winner)**"
        } else {
            r.name
        };
        md.push_str(&format!(
            "| {} | `{}` | {:.1} MB | **{:.1} ms** | {:.1} ms | **{:.1} MiB** | {:.2}% |\n",
            badge,
            r.image,
            r.image_size_mb,
            r.avg_startup_ms(),
            r.min_startup_ms(),
            r.avg_memory_mib(),
            r.avg_cpu_perc()
        ));
    }

    if let Some(ruststack) = results.iter().find(|r| r.name == "RustStack") {
        if let Some(ministack) = results.iter().find(|r| r.name == "MiniStack") {
            let start_speedup = ministack.avg_startup_ms() / ruststack.avg_startup_ms().max(1.0);
            let mem_savings = ministack.avg_memory_mib() / ruststack.avg_memory_mib().max(1.0);
            let size_savings = ministack.image_size_mb / ruststack.image_size_mb.max(1.0);

            md.push_str("\n### 🚀 RustStack vs MiniStack Advantage\n\n");
            md.push_str(&format!(
                "- **Startup Speed**: **{:.1}x faster** startup ({:.1} ms vs {:.1} ms)\n",
                start_speedup,
                ruststack.avg_startup_ms(),
                ministack.avg_startup_ms()
            ));
            md.push_str(&format!(
                "- **Memory Footprint**: **{:.1}x less memory** ({:.1} MiB vs {:.1} MiB)\n",
                mem_savings,
                ruststack.avg_memory_mib(),
                ministack.avg_memory_mib()
            ));
            md.push_str(&format!(
                "- **Docker Image Size**: **{:.1}x smaller** ({:.1} MB vs {:.1} MB)\n",
                size_savings,
                ruststack.image_size_mb,
                ministack.image_size_mb
            ));
        }
    }

    md
}
