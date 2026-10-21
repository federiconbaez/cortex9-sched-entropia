use fnbr_sched_entropia::{score, shannon_entropy, ScoreConfig, entropy_norm};

#[test]
fn empty_prompt_zero_entropy() {
    assert_eq!(shannon_entropy(b""), 0.0);
}

#[test]
fn repeated_bytes_low_entropy() {
    let e = shannon_entropy(&[b'a'; 1000]);
    assert!(e < 0.01, "esperado ~0, got {e}");
}

#[test]
fn diverse_bytes_high_entropy() {
    let mut buf = Vec::new();
    for i in 0..=255u8 {
        buf.push(i);
    }
    let e = shannon_entropy(&buf);
    assert!((e - 8.0).abs() < 1e-9, "esperado 8 bits, got {e}");
    assert!((entropy_norm(&buf) - 1.0).abs() < 1e-9);
}

#[test]
fn urgency_dominates_weight() {
    let cfg = ScoreConfig::default();
    let low = score(b"aaaa", 0.0, &cfg);
    let high = score(b"aaaa", 1.0, &cfg);
    assert!(high.cpu_weight > low.cpu_weight);
    assert!(high.cpu_weight <= 10_000);
}

#[test]
fn weight_in_configured_range() {
    let cfg = ScoreConfig::default();
    let p = score(b"hello world", 0.5, &cfg);
    assert!(p.cpu_weight >= cfg.weight_min && p.cpu_weight <= cfg.weight_max);
}
