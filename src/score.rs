/// Configuración del scoring. Los pesos suman 1.0.
#[derive(Debug, Clone)]
pub struct ScoreConfig {
    pub entropy_weight: f64,
    pub urgency_weight: f64,
    /// Mapeo de score [0,1] a `cpu.weight` de cgroup v2 (rango 1..=10000).
    pub weight_min: u32,
    pub weight_max: u32,
}

impl Default for ScoreConfig {
    fn default() -> Self {
        Self {
            entropy_weight: 0.4,
            urgency_weight: 0.6,
            weight_min: 50,
            weight_max: 1000,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Priority {
    /// Score normalizado en [0, 1].
    pub score: f64,
    /// Entropía de Shannon del prompt en bits/byte.
    pub entropy_bits: f64,
    /// Urgencia de consenso en [0, 1].
    pub urgency: f64,
    /// Valor de `cpu.weight` a aplicar.
    pub cpu_weight: u32,
}

/// Entropía de Shannon (bits/símbolo) sobre la distribución de bytes del prompt.
pub fn shannon_entropy(prompt: &[u8]) -> f64 {
    if prompt.is_empty() {
        return 0.0;
    }
    let mut hist = [0u64; 256];
    for &b in prompt {
        hist[b as usize] += 1;
    }
    let n = prompt.len() as f64;
    hist.iter()
        .filter(|&&c| c > 0)
        .map(|&c| {
            let p = c as f64 / n;
            -p * p.log2()
        })
        .sum()
}

/// Normaliza la entropía a [0, 1] respecto del máximo teórico (8 bits/byte).
pub fn entropy_norm(prompt: &[u8]) -> f64 {
    (shannon_entropy(prompt) / 8.0).clamp(0.0, 1.0)
}

/// Combina entropía normalizada y urgencia en un score y un `cpu.weight`.
///
/// Hipótesis de diseño (explícita): un prompt de alta entropía (más diverso,
/// menos repetitivo) y alta urgencia de consenso merecen más CPU. La urgencia
/// la aporta el orquestador; este módulo no la inventa.
pub fn score(prompt: &[u8], urgency: f64, cfg: &ScoreConfig) -> Priority {
    let urgency = urgency.clamp(0.0, 1.0);
    let e = entropy_norm(prompt);
    let s = (cfg.entropy_weight * e + cfg.urgency_weight * urgency).clamp(0.0, 1.0);
    let span = (cfg.weight_max - cfg.weight_min) as f64;
    let w = cfg.weight_min + (s * span).round() as u32;
    Priority {
        score: s,
        entropy_bits: shannon_entropy(prompt),
        urgency,
        cpu_weight: w.clamp(1, 10_000),
    }
}
