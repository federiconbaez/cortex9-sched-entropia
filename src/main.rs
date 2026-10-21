//! CLI:
//!   fnbr-sched score --prompt "..." --urgency 0.8
//!   fnbr-sched apply --name agente-a --prompt "..." --urgency 0.8
//!   fnbr-sched bench --seconds 2
use fnbr_sched_entropia::{cgroup, score, ScoreConfig};
use std::env;
use std::io::{self, Write};
use std::process::{Command, ExitCode, Stdio};
use std::thread;
use std::time::Instant;

fn usage() {
    eprintln!(
        "uso:\n  fnbr-sched score  --prompt TEXT --urgency F\n  fnbr-sched apply  --name NAME --prompt TEXT --urgency F\n  fnbr-sched bench  [--seconds N]"
    );
}

fn parse_kv(args: &[String], key: &str) -> Option<String> {
    args.windows(2)
        .find(|w| w[0] == key)
        .map(|w| w[1].clone())
}

fn cmd_score(args: &[String]) -> ExitCode {
    let prompt = parse_kv(args, "--prompt").unwrap_or_default();
    let urgency: f64 = parse_kv(args, "--urgency")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.5);
    let p = score(prompt.as_bytes(), urgency, &ScoreConfig::default());
    println!(
        "{{\"score\":{:.4},\"entropy_bits\":{:.4},\"urgency\":{:.4},\"cpu_weight\":{}}}",
        p.score, p.entropy_bits, p.urgency, p.cpu_weight
    );
    ExitCode::SUCCESS
}

fn cmd_apply(args: &[String]) -> ExitCode {
    let name = match parse_kv(args, "--name") {
        Some(n) => n,
        None => {
            eprintln!("falta --name");
            return ExitCode::from(2);
        }
    };
    let prompt = parse_kv(args, "--prompt").unwrap_or_default();
    let urgency: f64 = parse_kv(args, "--urgency")
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.5);
    let p = score(prompt.as_bytes(), urgency, &ScoreConfig::default());
    let root = cgroup::work_root();
    match cgroup::AgentCgroup::create(&root, &name) {
        Ok(cg) => match cg.set_cpu_weight(p.cpu_weight) {
            Ok(()) => {
                println!(
                    "{{\"cgroup\":\"{}\",\"cpu_weight\":{},\"score\":{:.4},\"note\":\"peso escrito; attach de PID puede fallar con EOPNOTSUPP en este contenedor\"}}",
                    cg.path().display(),
                    p.cpu_weight,
                    p.score
                );
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("no pude escribir cpu.weight: {e}");
                ExitCode::from(1)
            }
        },
        Err(e) => {
            eprintln!("no pude crear cgroup bajo {}: {e}", root.display());
            ExitCode::from(1)
        }
    }
}

/// Dos procesos en la misma CPU (`taskset -c 0`) con `nice` distinto.
fn busy_process(nice: i32, secs: u64) -> io::Result<f64> {
    let code = format!(
        "import time\nend=time.time()+{secs}\nn=0\nwhile time.time()<end:\n n+=1\nprint(n)\n"
    );
    let t0 = Instant::now();
    let out = Command::new("taskset")
        .args([
            "-c",
            "0",
            "nice",
            "-n",
            &nice.to_string(),
            "--",
            "python3",
            "-c",
            &code,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()?;
    let elapsed = t0.elapsed().as_secs_f64();
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(io::Error::other(format!("nice {nice}: {err}")));
    }
    let n: f64 = String::from_utf8_lossy(&out.stdout)
        .trim()
        .parse()
        .unwrap_or(0.0);
    let _ = writeln!(io::stderr(), "nice={nice} iters={n} wall={elapsed:.3}s");
    Ok(n / elapsed.max(1e-6))
}

fn cmd_bench(args: &[String]) -> ExitCode {
    let secs: u64 = parse_kv(args, "--seconds")
        .and_then(|s| s.parse().ok())
        .unwrap_or(4);

    let s1 = secs;
    let s2 = secs;
    let t_low = thread::spawn(move || busy_process(15, s1));
    let t_high = thread::spawn(move || busy_process(-10, s2));
    let r_low = t_low.join().unwrap_or(Err(io::Error::other("join")));
    let r_high = t_high.join().unwrap_or(Err(io::Error::other("join")));

    match (r_low, r_high) {
        (Ok(ips_low), Ok(ips_high)) => {
            let ratio = ips_high / ips_low.max(1.0);
            println!(
                "{{\"iters_per_s_low_nice15\":{:.0},\"iters_per_s_high_nice_m10\":{:.0},\"ratio_high_over_low\":{:.3},\"backend\":\"taskset0+nice\",\"note\":\"cgroup.procs EOPNOTSUPP; sched_ext ausente; contención forzada en CPU0\"}}",
                ips_low, ips_high, ratio
            );
            if ratio > 1.2 {
                ExitCode::SUCCESS
            } else {
                eprintln!("sin prioridad efectiva medible (ratio={ratio:.2}); ¿corriste con sudo?");
                ExitCode::from(1)
            }
        }
        (Err(e), _) | (_, Err(e)) => {
            eprintln!("bench falló: {e}");
            ExitCode::from(1)
        }
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("score") => cmd_score(&args),
        Some("apply") => cmd_apply(&args),
        Some("bench") => cmd_bench(&args),
        _ => {
            usage();
            ExitCode::from(2)
        }
    }
}
