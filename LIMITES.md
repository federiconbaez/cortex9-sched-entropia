# Límites — 01 sched-entropía (v0.2)

Lo que **sí** está demostrado y lo que **no**, sin maquillaje.

## Demostrado
- El `struct_ops` sched_ext `fnbr_entropia` **carga y se adjunta** en una VM QEMU con kernel Ubuntu
  `6.14.0-37-generic` (`/sys/kernel/sched_ext/state=enabled`, `root/ops=fnbr_entropia`, mensaje en dmesg).
- El espacio de usuario en Rust escribe `tgid → peso` en el mapa BPF a partir del scoring
  de entropía + urgencia, y el planificador usa ese peso (contador `enqueue_con_peso` > 0).
- Con 4 agentes en una CPU, la cuota medida sigue a los pesos (A/D = 15,33× vs 1,00× sin sched_ext).

## No demostrado / límites
1. **El host de esta computadora no puede cargar sched_ext**: kernel `6.12.94+` sin
   `CONFIG_SCHED_CLASS_EXT` (no existe `/sys/kernel/sched_ext`). Debian 13 tampoco lo trae en sus
   kernels 6.12 (`linux-image-6.12.107+deb13-amd64` y `-cloud`: `# CONFIG_SCHED_CLASS_EXT is not set`).
2. **La VM corre con TCG (emulación), no con KVM.** `/dev/kvm` existe, pero crear una vCPU dispara
   `kernel BUG at arch/x86/kvm/x86.c:702` (`kvm_spurious_fault`) en el host (KVM anidado roto).
   Se dejó de intentar KVM para no seguir provocando BUGs. Las **proporciones** de CPU son válidas;
   la latencia y el rendimiento absolutos no son representativos de hardware real.
3. Las cuotas medidas se desvían algo del ideal (A 50,6 % vs 53,1 %; A/D 15,3× vs 16,9×): el
   planificador limita el crédito acumulado a un slice (`vtime_now - SCX_SLICE_DFL`) y hay ruido
   de contabilidad por ticks (100 Hz). No hay ajuste fino de esto.
4. Los "agentes" del bench son bucles `sh` que simulan carga; no son agentes LLM reales. El peso
   se asigna por proceso (tgid), no por hilo, y se escribe una vez (no hay demonio que re-puntúe
   en vivo a medida que cambia el prompt o el consenso).
5. La urgencia de consenso la aporta quien llama (`--urgency`); el sistema no la calcula.
6. Política simple: una DSQ global (sin localidad NUMA/LLC, sin preferencia de CPU más allá de
   `scx_bpf_select_cpu_dfl`). Si el programa BPF falla, el kernel vuelve a EEVDF solo (garantía de sched_ext).
7. Kfuncs de Linux ≥ 6.13 (`scx_bpf_dsq_insert*`, `scx_bpf_dsq_move_to_local`); en 6.12 habría que
   usar los nombres viejos (`scx_bpf_dispatch*`, `scx_bpf_consume`).
8. `vmlinux.h`, el `.bpf.o`, el skeleton y el binario del cargador **no** se versionan: los regenera
   `vm/preparar.sh` desde el `.deb` del kernel (sha256 `990f51c2…7265`).
