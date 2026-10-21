//! Scoring de prioridad para agentes locales.
//!
//! La entropía del prompt y la urgencia del consenso se calculan en espacio
//! de usuario. El backend aplica esa prioridad: en kernels con `sched_ext`
//! vía el struct_ops `scx/scx_entropia` (mapa `agent_weight`, ver `bpfmap`);
//! en kernels sin `sched_ext` vía `cpu.weight` de cgroup v2 o `nice`.

pub mod bpfmap;
pub mod cgroup;
pub mod score;

pub use score::{Priority, ScoreConfig, entropy_norm, score, shannon_entropy};
