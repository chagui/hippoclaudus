use anyhow::Result;
use chrono::{DateTime, Datelike, Local, NaiveDate};
use std::collections::BTreeMap;
use std::path::Path;

use hippoclaudus::analytics::{compute_distribution, Distribution};
use hippoclaudus::config::Config;
use hippoclaudus::pricing::{cost_usd, model_family};
use hippoclaudus::session::{extract_session_metadata, ActiveSessionInfo};
use hippoclaudus::{discover_active_sessions, discover_sessions};

/// Per-session sample: everything the aggregations need in one pass.
struct Sample {
    date: NaiveDate,
    model_family: &'static str,
    input_tokens: u64,
    output_tokens: u64,
    cache_read_tokens: u64,
    cache_create_tokens: u64,
    total_tokens: u64,
    cost_usd: f64,
    turn_count: u64,
    avg_turn_duration_ms: u64,
    total_duration_ms: u64,
    agent_time_ms: u64,
    user_time_ms: u64,
    agent_time_pct: f64,
    write_count: u64,
    edit_count: u64,
    bash_count: u64,
    files_touched_count: u64,
}

impl Sample {
    fn from_info(info: &ActiveSessionInfo) -> Self {
        let date = parse_local_date(&info.started_at).unwrap_or_else(today);
        let total_tokens = info.total_input_tokens + info.total_output_tokens;
        let cost = cost_usd(
            &info.model,
            info.total_input_tokens,
            info.total_output_tokens,
            info.total_cache_read_tokens,
            info.total_cache_creation_tokens,
        );
        let (total_ms, agent_ms, user_ms, agent_pct) = info
            .timing
            .as_ref()
            .map(|t| {
                (
                    t.total_duration_ms,
                    t.agent_time_ms,
                    t.user_time_ms,
                    t.agent_time_pct,
                )
            })
            .unwrap_or((0, 0, 0, 0.0));

        Self {
            date,
            model_family: model_family(&info.model),
            input_tokens: info.total_input_tokens,
            output_tokens: info.total_output_tokens,
            cache_read_tokens: info.total_cache_read_tokens,
            cache_create_tokens: info.total_cache_creation_tokens,
            total_tokens,
            cost_usd: cost,
            turn_count: info.turn_count as u64,
            avg_turn_duration_ms: info.avg_turn_duration_ms,
            total_duration_ms: total_ms,
            agent_time_ms: agent_ms,
            user_time_ms: user_ms,
            agent_time_pct: agent_pct,
            write_count: info.write_count as u64,
            edit_count: info.edit_count as u64,
            bash_count: info.bash_count as u64,
            files_touched_count: info.files_touched_count as u64,
        }
    }
}

fn parse_local_date(ts: &str) -> Option<NaiveDate> {
    DateTime::parse_from_rfc3339(ts)
        .ok()
        .map(|dt| dt.with_timezone(&Local).date_naive())
}

fn today() -> NaiveDate {
    Local::now().date_naive()
}

fn collect_samples(config: &Config, days: u32) -> Vec<Sample> {
    let all_paths = discover_sessions(config, Some(days));
    let active_paths = discover_active_sessions(config);
    let paths: Vec<_> = all_paths.iter().chain(active_paths.iter()).collect();
    paths
        .iter()
        .filter_map(|p| extract_session_metadata(p.as_ref() as &Path).ok())
        .map(|info| Sample::from_info(&info))
        .collect()
}

struct ModelAggregate {
    model: &'static str,
    session_count: u64,
    cost_usd: f64,
    total_tokens: u64,
    total_duration_ms: u64,
}

struct DailyAggregate {
    date: NaiveDate,
    session_count: u64,
    cost_usd: f64,
    total_tokens: u64,
    agent_time_ms: u64,
    user_time_ms: u64,
}

fn compute_by_model(samples: &[Sample]) -> Vec<ModelAggregate> {
    let mut map: BTreeMap<&'static str, ModelAggregate> = BTreeMap::new();
    for s in samples {
        let entry = map.entry(s.model_family).or_insert(ModelAggregate {
            model: s.model_family,
            session_count: 0,
            cost_usd: 0.0,
            total_tokens: 0,
            total_duration_ms: 0,
        });
        entry.session_count += 1;
        entry.cost_usd += s.cost_usd;
        entry.total_tokens += s.total_tokens;
        entry.total_duration_ms += s.total_duration_ms;
    }
    // Stable order: opus, sonnet, haiku, other. BTreeMap alpha order happens to be close;
    // emit in descending cost order instead so the UI shows the biggest bucket first.
    let mut out: Vec<_> = map.into_values().collect();
    out.sort_by(|a, b| {
        b.cost_usd
            .partial_cmp(&a.cost_usd)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out
}

fn compute_daily(samples: &[Sample], days: u32) -> Vec<DailyAggregate> {
    let mut map: BTreeMap<NaiveDate, DailyAggregate> = BTreeMap::new();
    for s in samples {
        let entry = map.entry(s.date).or_insert(DailyAggregate {
            date: s.date,
            session_count: 0,
            cost_usd: 0.0,
            total_tokens: 0,
            agent_time_ms: 0,
            user_time_ms: 0,
        });
        entry.session_count += 1;
        entry.cost_usd += s.cost_usd;
        entry.total_tokens += s.total_tokens;
        entry.agent_time_ms += s.agent_time_ms;
        entry.user_time_ms += s.user_time_ms;
    }

    // Zero-fill the window so the chart has a continuous x-axis.
    let end = today();
    let start = end
        .checked_sub_signed(chrono::Duration::days(days as i64 - 1))
        .unwrap_or(end);
    let mut day = start;
    while day <= end {
        map.entry(day).or_insert(DailyAggregate {
            date: day,
            session_count: 0,
            cost_usd: 0.0,
            total_tokens: 0,
            agent_time_ms: 0,
            user_time_ms: 0,
        });
        day = day.succ_opt().unwrap_or(day);
    }

    map.into_values().collect()
}

fn distribution_json(d: &Distribution) -> serde_json::Value {
    serde_json::json!({
        "count": d.count,
        "min": d.min,
        "max": d.max,
        "mean": d.mean,
        "p50": d.p50,
        "p90": d.p90,
        "p95": d.p95,
        "p99": d.p99,
    })
}

pub fn cmd_analytics(config: &Config, days: u32, json: bool) -> Result<()> {
    let samples = collect_samples(config, days);

    // Extract parallel vectors per metric for percentile computation.
    let input_tokens: Vec<f64> = samples.iter().map(|s| s.input_tokens as f64).collect();
    let output_tokens: Vec<f64> = samples.iter().map(|s| s.output_tokens as f64).collect();
    let cache_read: Vec<f64> = samples.iter().map(|s| s.cache_read_tokens as f64).collect();
    let cache_create: Vec<f64> = samples
        .iter()
        .map(|s| s.cache_create_tokens as f64)
        .collect();
    let total_tokens: Vec<f64> = samples.iter().map(|s| s.total_tokens as f64).collect();
    let cost: Vec<f64> = samples.iter().map(|s| s.cost_usd).collect();
    let turn_count: Vec<f64> = samples.iter().map(|s| s.turn_count as f64).collect();
    let avg_turn: Vec<f64> = samples
        .iter()
        .map(|s| s.avg_turn_duration_ms as f64)
        .collect();
    let total_duration: Vec<f64> = samples.iter().map(|s| s.total_duration_ms as f64).collect();
    let agent_time: Vec<f64> = samples.iter().map(|s| s.agent_time_ms as f64).collect();
    let user_time: Vec<f64> = samples.iter().map(|s| s.user_time_ms as f64).collect();
    let agent_pct: Vec<f64> = samples.iter().map(|s| s.agent_time_pct).collect();
    let writes: Vec<f64> = samples.iter().map(|s| s.write_count as f64).collect();
    let edits: Vec<f64> = samples.iter().map(|s| s.edit_count as f64).collect();
    let bashes: Vec<f64> = samples.iter().map(|s| s.bash_count as f64).collect();
    let files: Vec<f64> = samples
        .iter()
        .map(|s| s.files_touched_count as f64)
        .collect();

    let totals_cost_usd: f64 = cost.iter().sum();
    let totals_total_tokens: u64 = samples.iter().map(|s| s.total_tokens).sum();
    let totals_duration_ms: u64 = samples.iter().map(|s| s.total_duration_ms).sum();

    let by_model = compute_by_model(&samples);
    let daily = compute_daily(&samples, days);

    if json {
        let distributions = serde_json::json!({
            "input_tokens": distribution_json(&compute_distribution(&input_tokens)),
            "output_tokens": distribution_json(&compute_distribution(&output_tokens)),
            "cache_read_tokens": distribution_json(&compute_distribution(&cache_read)),
            "cache_create_tokens": distribution_json(&compute_distribution(&cache_create)),
            "total_tokens": distribution_json(&compute_distribution(&total_tokens)),
            "cost_usd": distribution_json(&compute_distribution(&cost)),
            "turn_count": distribution_json(&compute_distribution(&turn_count)),
            "avg_turn_duration_ms": distribution_json(&compute_distribution(&avg_turn)),
            "total_duration_ms": distribution_json(&compute_distribution(&total_duration)),
            "agent_time_ms": distribution_json(&compute_distribution(&agent_time)),
            "user_time_ms": distribution_json(&compute_distribution(&user_time)),
            "agent_time_pct": distribution_json(&compute_distribution(&agent_pct)),
            "write_count": distribution_json(&compute_distribution(&writes)),
            "edit_count": distribution_json(&compute_distribution(&edits)),
            "bash_count": distribution_json(&compute_distribution(&bashes)),
            "files_touched_count": distribution_json(&compute_distribution(&files)),
        });

        let by_model_json: Vec<_> = by_model
            .iter()
            .map(|m| {
                serde_json::json!({
                    "model": m.model,
                    "session_count": m.session_count,
                    "cost_usd": m.cost_usd,
                    "total_tokens": m.total_tokens,
                    "total_duration_ms": m.total_duration_ms,
                })
            })
            .collect();

        let daily_json: Vec<_> = daily
            .iter()
            .map(|d| {
                serde_json::json!({
                    "date": d.date.to_string(),
                    "session_count": d.session_count,
                    "cost_usd": d.cost_usd,
                    "total_tokens": d.total_tokens,
                    "agent_time_ms": d.agent_time_ms,
                    "user_time_ms": d.user_time_ms,
                })
            })
            .collect();

        let out = serde_json::json!({
            "days": days,
            "session_count": samples.len(),
            "totals": {
                "cost_usd": totals_cost_usd,
                "total_tokens": totals_total_tokens,
                "total_duration_ms": totals_duration_ms,
            },
            "distributions": distributions,
            "by_model": by_model_json,
            "daily": daily_json,
        });
        println!("{}", serde_json::to_string_pretty(&out).unwrap());
    } else {
        print_text(
            days,
            samples.len(),
            totals_cost_usd,
            totals_total_tokens,
            totals_duration_ms,
            &cost,
            &total_tokens,
            &turn_count,
            &avg_turn,
            &total_duration,
            &agent_pct,
            &writes,
            &edits,
            &bashes,
            &files,
            &by_model,
            &daily,
        );
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn print_text(
    days: u32,
    session_count: usize,
    total_cost: f64,
    total_tokens: u64,
    total_duration_ms: u64,
    cost: &[f64],
    tokens: &[f64],
    turn_count: &[f64],
    avg_turn: &[f64],
    total_duration: &[f64],
    agent_pct: &[f64],
    writes: &[f64],
    edits: &[f64],
    bashes: &[f64],
    files: &[f64],
    by_model: &[ModelAggregate],
    daily: &[DailyAggregate],
) {
    println!("=== Session Analytics (last {} days) ===", days);
    println!(
        "Sessions: {} | Total cost: ${:.2} | Total tokens: {} | Total duration: {}",
        session_count,
        total_cost,
        format_tokens(total_tokens),
        format_duration_ms(total_duration_ms),
    );

    if session_count == 0 {
        println!("(no sessions in window)");
        return;
    }

    println!();
    println!("Distributions (p50 / p90 / p95 / p99 / max):");
    print_row("cost ($)", cost, format_cost);
    print_row("tokens", tokens, |v| format_tokens(v as u64));
    print_row("turns", turn_count, |v| format!("{}", v as u64));
    print_row("avg turn", avg_turn, |v| format_duration_ms(v as u64));
    print_row("duration", total_duration, |v| format_duration_ms(v as u64));
    print_row("agent %", agent_pct, |v| format!("{:.0}%", v));
    print_row("writes", writes, |v| format!("{}", v as u64));
    print_row("edits", edits, |v| format!("{}", v as u64));
    print_row("bash", bashes, |v| format!("{}", v as u64));
    print_row("files", files, |v| format!("{}", v as u64));

    if !by_model.is_empty() {
        println!();
        println!("By model:");
        for m in by_model {
            println!(
                "  {:<7} {} sessions | ${:.2} | {} tokens | {}",
                m.model,
                m.session_count,
                m.cost_usd,
                format_tokens(m.total_tokens),
                format_duration_ms(m.total_duration_ms),
            );
        }
    }

    // Small ASCII daily bar (cost). Scales to the max daily cost in the window.
    let max_daily = daily.iter().map(|d| d.cost_usd).fold(0.0_f64, f64::max);
    if max_daily > 0.0 {
        println!();
        println!(
            "Daily cost ($, bars scaled to window max ${:.2}):",
            max_daily
        );
        for d in daily {
            let blocks = ((d.cost_usd / max_daily) * 30.0).round() as usize;
            let bar: String = "█".repeat(blocks);
            println!(
                "  {:04}-{:02}-{:02} {:>6.2} {}",
                d.date.year(),
                d.date.month(),
                d.date.day(),
                d.cost_usd,
                bar,
            );
        }
    }
}

fn print_row<F: Fn(f64) -> String>(label: &str, samples: &[f64], fmt: F) {
    let d = compute_distribution(samples);
    println!(
        "  {:<10} p50={:>8} p90={:>8} p95={:>8} p99={:>8} max={:>8}",
        label,
        fmt(d.p50),
        fmt(d.p90),
        fmt(d.p95),
        fmt(d.p99),
        fmt(d.max),
    );
}

fn format_tokens(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.0}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

fn format_cost(v: f64) -> String {
    if v >= 0.01 {
        format!("${:.2}", v)
    } else if v > 0.0 {
        "<$0.01".to_string()
    } else {
        "$0".to_string()
    }
}

fn format_duration_ms(ms: u64) -> String {
    let total_secs = ms / 1000;
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;
    if hours > 0 {
        format!("{}h{}m", hours, minutes)
    } else if minutes > 0 {
        format!("{}m{}s", minutes, seconds)
    } else {
        format!("{}s", seconds)
    }
}
