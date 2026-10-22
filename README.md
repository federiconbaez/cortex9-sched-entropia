# 01 — Planificador por entropía / consenso

> **ES:** Planificador `sched_ext` (eBPF + Rust) que pondera hilos de agentes por entropía del prompt y urgencia. Compilar/probar: `cargo build --release && cargo test --release`.
> **EN:** `sched_ext` (eBPF + Rust) scheduler weighting agent threads by prompt entropy and urgency. Build/test: `cargo build --release && cargo test --release`.
>
> **Estado / Status:** cargado en una VM QEMU (kernel 6.14) y 7/7 tests; experimento de valor **no auditado** / value experiment **not audited**. Licencia / License: Apache-2.0.


Dueña: **Federica Research Sentinel**. Pedido de Federico, 20/10/2026.

## Qué pediste
Un planificador `sched_ext` (eBPF) que priorice hilos de agentes según la
entropía de sus prompts o la urgencia de su consenso epistémico.

## v0.2 — sched_ext REAL en VM (20/10/2026)

**Qué cargó:** el `struct_ops` `fnbr_entropia` (`scx/scx_entropia.bpf.c`) se adjuntó de verdad
en una VM QEMU con kernel Ubuntu `6.14.0-37-generic` (`CONFIG_SCHED_CLASS_EXT=y`).
Evidencia en el log: `state=enabled ops=fnbr_entropia` y en `dmesg`:
`sched_ext: BPF scheduler "fnbr_entropia" enabled`. Se cargó y descargó 5 veces (`enable_seq=1..5`).

**Cómo funciona:**
- `scx/scx_entropia.bpf.c`: planificador de tiempo virtual ponderado (patrón `scx_simple`) con DSQ compartida.
  El peso de cada tarea sale del mapa `agent_weight` (tgid → peso); si no está, usa `p->scx.weight`.
- `scx/scx_entropia.c`: cargador libbpf; fija el mapa en `/sys/fs/bpf/fnbr_agent_weight`.
- `fnbr-sched scx-set --pid P --prompt ... --urgency U` (Rust, `src/bpfmap.rs`): calcula el peso con el
  scoring de entropía + urgencia y lo escribe en el mapa con la syscall `bpf(2)` (y lo relee para verificar).

**Qué se midió (dentro de la VM):** 4 agentes compitiendo en la CPU 1, 10 s por fase, 5 repeticiones.
Pesos del scoring: A=846 (prompt diverso, urgencia 0,95), B=514, C=182, D=50 (prompt `aaaa…`, urgencia 0).

| agente | peso | cuota esperada | sin sched_ext (EEVDF) | con `fnbr_entropia` |
|---|---|---|---|---|
| A | 846 | 53,14 % | 24,99 % | **50,58 % ± 0,09** |
| B | 514 | 32,29 % | 24,97 % | **33,99 % ± 0,15** |
| C | 182 | 11,43 % | 25,03 % | **12,13 % ± 0,06** |
| D |  50 |  3,14 % | 25,01 % | **3,31 % ± 0,14** |

Ratio de CPU alta/baja prioridad (A/D): **15,33×** (min 14,51, max 16,39; ideal 16,92). Sin sched_ext: **1,00×**.
Log crudo: `evidencia/vm-bench-20261021-0235.log`; resumen: `evidencia/RESUMEN.txt`.

**Reproducir:**
```bash
./vm/preparar.sh        # baja kernel (sha256 verificado), genera vmlinux.h, compila, arma initramfs
T=10 REPS=5 ./vm/correr-vm.sh | tee evidencia/run.log
python3 vm/resumir.py evidencia/run.log
```

Límites: ver `LIMITES.md` (en esta computadora el host **sigue sin poder** cargar sched_ext, y la VM
corre con TCG, no KVM).

## Qué hay (v0.1)
- Scoring en Rust: entropía de Shannon del prompt + urgencia ∈ [0,1] → `cpu_weight`.
- Escritura de `cpu.weight` en cgroup v2 (el attach de PID falla aquí con EOPNOTSUPP).
- Prioridad **medida** con `taskset -c 0` + `nice` (fallback verificable).
- Objeto eBPF compilado: `bpf/sched_entropia.bpf.o` (stub socket + mapa `agent_weight`). **Superado por `scx/` en v0.2.**
- CLI: `fnbr-sched score|apply|bench`.

## Estado verificado
- `cargo build --release`: limpio.
- `cargo clippy --all-targets -- -D warnings`: limpio.
- `cargo test --release`: 7/7 (5 de scoring + 2 de `bpfmap`, v0.2).
- `clang -target bpf` produce `bpf/sched_entropia.bpf.o`.
- Bench (sudo, 4 s, CPU0): ratio nice(-10)/nice(15) ≈ **3.04** (> 1.2). Prioridad efectiva sí se mide.

## Lo que NO está verificado (en el host)
- **`sched_ext` no existe en el host** (`/sys/kernel/sched_ext` ausente). El scheduler eBPF pedido
  no se puede cargar en esta computadora. El `.bpf.o` es un stub de toolchain, no un
  `struct_ops` de sched_ext. (El struct_ops real está en `scx/` y se verificó en VM, ver v0.2.)
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
