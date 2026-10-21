# 01 — Planificador por entropía / consenso

Dueña: **Federica Research Sentinel**. Pedido de Federico, 20/10/2026.

## Qué pediste
Un planificador `sched_ext` (eBPF) que priorice hilos de agentes según la
entropía de sus prompts o la urgencia de su consenso epistémico.

## Qué hay (v0.1)
- Scoring en Rust: entropía de Shannon del prompt + urgencia ∈ [0,1] → `cpu_weight`.
- Escritura de `cpu.weight` en cgroup v2 (el attach de PID falla aquí con EOPNOTSUPP).
- Prioridad **medida** con `taskset -c 0` + `nice` (fallback verificable).
- Objeto eBPF compilado: `bpf/sched_entropia.bpf.o` (stub socket + mapa `agent_weight`).
- CLI: `fnbr-sched score|apply|bench`.

## Estado verificado
- `cargo build --release`: limpio.
- `cargo clippy --all-targets -- -D warnings`: limpio.
- `cargo test --release`: 5/5.
- `clang -target bpf` produce `bpf/sched_entropia.bpf.o`.
- Bench (sudo, 4 s, CPU0): ratio nice(-10)/nice(15) ≈ **3.04** (> 1.2). Prioridad efectiva sí se mide.

## Lo que NO está verificado
- **`sched_ext` no existe** (`/sys/kernel/sched_ext` ausente). El scheduler eBPF pedido
  no se puede cargar en esta computadora. El `.bpf.o` es un stub de toolchain, no un
  `struct_ops` de sched_ext.
- Mover PIDs a cgroups (`cgroup.procs`) devuelve **EOPNOTSUPP** en este contenedor.
- La hipótesis “más entropía ⇒ más CPU” es de diseño; los pesos están en `ScoreConfig`.

## Cómo correr
```bash
cargo test --release
cargo build --release
./target/release/fnbr-sched score --prompt "hola" --urgency 0.8
sudo ./target/release/fnbr-sched apply --name agente-a --prompt "..." --urgency 0.9
sudo ./target/release/fnbr-sched bench --seconds 4
```
