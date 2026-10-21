//! Scoring de prioridad para agentes locales.
//!
//! La entropía del prompt y la urgencia del consenso se calculan en espacio
//! de usuario. El backend aplica esa prioridad: en kernels con `sched_ext`
//! (no disponible aquí) vía eBPF; en esta máquina vía `cpu.weight` de cgroup v2.

pub mod cgroup;
pub mod score;

pub use score::{Priority, ScoreConfig, entropy_norm, score, shannon_entropy};
