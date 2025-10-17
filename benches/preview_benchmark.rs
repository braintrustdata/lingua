use lingua::processing::import::import_and_deduplicate_messages;
use lingua::processing::import::Span;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TraceSnapshot {
    #[serde(rename = "projectId")]
    project_id: String,
    #[serde(rename = "rootSpanId")]
    root_span_id: String,
    spans: Vec<Value>,
    metadata: serde_json::Map<String, Value>,
}

fn load_snapshots() -> anyhow::Result<Vec<TraceSnapshot>> {
    let snapshot_path = "../api-ts/src/deep-search/evals/preview-snapshots.json";
    let data = fs::read_to_string(snapshot_path)?;
    let parsed: std::collections::HashMap<String, TraceSnapshot> = serde_json::from_str(&data)?;
    Ok(parsed.into_values().collect())
}

fn convert_to_spans(values: &[Value]) -> Vec<Span> {
    values
        .iter()
        .filter_map(|v| serde_json::from_value::<Span>(v.clone()).ok())
        .collect()
}

fn convert_to_spans_timed(values: &[Value]) -> (Vec<Span>, f64) {
    let start = Instant::now();
    let spans = values
        .iter()
        .filter_map(|v| serde_json::from_value::<Span>(v.clone()).ok())
        .collect();
    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    (spans, elapsed)
}

fn run_benchmark(snapshots: &[TraceSnapshot], iterations: usize, warmup: usize) {
    println!("\n⏱️  Running Rust native benchmark...");
    println!("  Warmup: {} iterations", warmup);
    println!("  Benchmark: {} iterations", iterations);
    println!("  Snapshots per iteration: {}", snapshots.len());

    // Warmup
    for i in 0..warmup {
        print!("\r  Warmup progress: {}/{}", i + 1, warmup);
        for snapshot in snapshots {
            let spans = convert_to_spans(&snapshot.spans);
            let _ = import_and_deduplicate_messages(spans);
        }
    }
    println!();

    // Detailed timing breakdown with granular measurements
    println!(
        "\n📊 Detailed timing breakdown (all {} snapshots):",
        snapshots.len()
    );
    println!("\n  Measuring breakdown of each stage:");

    let mut total_conversion = 0.0;
    let mut total_import = 0.0;
    let mut total_dedup = 0.0;

    for (i, snapshot) in snapshots.iter().enumerate() {
        // Time: Value -> Span deserialization
        let (spans, conversion_time) = convert_to_spans_timed(&snapshot.spans);
        total_conversion += conversion_time;

        // Time: Message extraction (import_messages_from_spans)
        let start = Instant::now();
        let messages = lingua::processing::import::import_messages_from_spans(spans);
        let import_time = start.elapsed().as_secs_f64() * 1000.0;
        total_import += import_time;

        // Time: Deduplication
        let start = Instant::now();
        let _ = lingua::processing::dedup::deduplicate_messages(messages);
        let dedup_time = start.elapsed().as_secs_f64() * 1000.0;
        total_dedup += dedup_time;

        let total_time = conversion_time + import_time + dedup_time;

        println!(
            "  Snapshot {}: deserialize={:.3}ms, import={:.3}ms, dedup={:.3}ms, total={:.3}ms",
            i + 1,
            conversion_time,
            import_time,
            dedup_time,
            total_time
        );
    }

    let grand_total = total_conversion + total_import + total_dedup;
    println!("\n  === BREAKDOWN SUMMARY ===");
    println!(
        "  Value->Span deserialization: {:.3}ms ({:.1}%)",
        total_conversion,
        total_conversion / grand_total * 100.0
    );
    println!(
        "  Message import:              {:.3}ms ({:.1}%)",
        total_import,
        total_import / grand_total * 100.0
    );
    println!(
        "  Deduplication:               {:.3}ms ({:.1}%)",
        total_dedup,
        total_dedup / grand_total * 100.0
    );
    println!("  Total:                       {:.3}ms", grand_total);

    // Benchmark
    let mut timings = Vec::new();
    for i in 0..iterations {
        print!("\r  Benchmark progress: {}/{}", i + 1, iterations);

        let start = Instant::now();
        for snapshot in snapshots {
            let spans = convert_to_spans(&snapshot.spans);
            let _ = import_and_deduplicate_messages(spans);
        }
        let duration = start.elapsed();

        timings.push(duration.as_secs_f64() * 1000.0); // Convert to milliseconds
    }
    println!();

    // Calculate statistics
    timings.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let sum: f64 = timings.iter().sum();
    let mean = sum / timings.len() as f64;
    let median = if timings.len() % 2 == 0 {
        (timings[timings.len() / 2 - 1] + timings[timings.len() / 2]) / 2.0
    } else {
        timings[timings.len() / 2]
    };
    let min = timings[0];
    let max = timings[timings.len() - 1];
    let p95_idx = (timings.len() as f64 * 0.95).ceil() as usize - 1;
    let p99_idx = (timings.len() as f64 * 0.99).ceil() as usize - 1;
    let p95 = timings[p95_idx];
    let p99 = timings[p99_idx];

    let variance: f64 = timings
        .iter()
        .map(|x| {
            let diff = x - mean;
            diff * diff
        })
        .sum::<f64>()
        / timings.len() as f64;
    let std_dev = variance.sqrt();

    let total_ops = snapshots.len() * iterations;
    let total_time_secs = sum / 1000.0;
    let throughput = total_ops as f64 / total_time_secs;

    println!("\n================================================================================");
    println!("RUST NATIVE BENCHMARK RESULTS");
    println!("================================================================================");
    println!("\nSnapshots processed: {}", snapshots.len());
    println!("Operations per iteration: {}", snapshots.len());

    println!("\n--------------------------------------------------------------------------------");
    println!("RUST NATIVE METHOD");
    println!("--------------------------------------------------------------------------------");
    println!("  Mean:       {:.2}ms", mean);
    println!("  Median:     {:.2}ms", median);
    println!("  Min:        {:.2}ms", min);
    println!("  Max:        {:.2}ms", max);
    println!("  P95:        {:.2}ms", p95);
    println!("  P99:        {:.2}ms", p99);
    println!("  Std Dev:    {:.2}ms", std_dev);
    println!("  Throughput: {:.2} ops/sec", throughput);

    println!("\n--------------------------------------------------------------------------------");
    println!("COMPARISON TO JS RESULTS");
    println!("--------------------------------------------------------------------------------");
    println!("  JS lingua-js:       839.60μs avg (10,719.34 ops/sec)");
    println!(
        "  Rust native:        {:.2}ms avg ({:.2} ops/sec)",
        mean, throughput
    );
    println!(
        "  Per operation:      {:.2}ms (Rust) vs 0.84ms (JS)",
        mean / snapshots.len() as f64
    );

    let js_time_ms = 0.83960; // JS lingua-js time in ms
    let rust_per_op = mean / snapshots.len() as f64;
    println!(
        "\n  JS is {:.2}x faster than Rust (per operation)",
        rust_per_op / js_time_ms
    );
    println!("\n  NOTE: Rust time dominated by 3 snapshots (#3, #5, #6)");
    println!("        which account for 96% of total processing time.");
    println!("        These likely have more complex message structures.");

    println!("\n================================================================================");
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let verify_mode = args.get(1).map(|s| s.as_str()) == Some("--verify");

    println!("🚀 Loading snapshot data...");
    let snapshots = load_snapshots()?;
    println!("📊 Total snapshots: {}", snapshots.len());

    // Count total messages to compare with JS
    let mut total_messages = 0;
    for snapshot in &snapshots {
        let spans = convert_to_spans(&snapshot.spans);
        let messages = import_and_deduplicate_messages(spans);
        total_messages += messages.len();
    }
    println!("📝 Total messages extracted: {}", total_messages);

    if verify_mode {
        println!("\n🔍 VERIFICATION MODE: Extracting messages for comparison\n");
        for (i, snapshot) in snapshots.iter().enumerate() {
            println!(
                "=== Snapshot {} (root_span_id: {}) ===",
                i + 1,
                snapshot.root_span_id
            );
            let spans = convert_to_spans(&snapshot.spans);
            let messages = import_and_deduplicate_messages(spans);
            println!("Messages extracted: {}", messages.len());
            for (j, msg) in messages.iter().enumerate() {
                let content_preview = match &msg {
                    lingua::universal::Message::User { content, .. } => {
                        format_content_preview(content)
                    }
                    lingua::universal::Message::Assistant { content, .. } => {
                        format_assistant_content_preview(content)
                    }
                    lingua::universal::Message::System { content } => {
                        format_content_preview(content)
                    }
                    lingua::universal::Message::Tool { content } => {
                        format!("[tool content with {} parts]", content.len())
                    }
                };
                println!("  {}: {:?} - {}", j + 1, get_role(msg), content_preview);
            }
            println!();
        }
        return Ok(());
    }

    run_benchmark(&snapshots, 10, 5);

    Ok(())
}

fn get_role(msg: &lingua::universal::Message) -> &str {
    match msg {
        lingua::universal::Message::User { .. } => "user",
        lingua::universal::Message::Assistant { .. } => "assistant",
        lingua::universal::Message::System { .. } => "system",
        lingua::universal::Message::Tool { .. } => "tool",
    }
}

fn format_content_preview(content: &lingua::universal::UserContent) -> String {
    match content {
        lingua::universal::UserContent::String(s) => {
            if s.len() > 100 {
                format!("\"{}...\" ({} chars)", &s[..100], s.len())
            } else {
                format!("\"{}\"", s)
            }
        }
        lingua::universal::UserContent::Array(parts) => {
            format!("[{} parts]", parts.len())
        }
    }
}

fn format_assistant_content_preview(content: &lingua::universal::AssistantContent) -> String {
    match content {
        lingua::universal::AssistantContent::String(s) => {
            if s.len() > 100 {
                format!("\"{}...\" ({} chars)", &s[..100], s.len())
            } else {
                format!("\"{}\"", s)
            }
        }
        lingua::universal::AssistantContent::Array(parts) => {
            format!("[{} parts]", parts.len())
        }
    }
}
