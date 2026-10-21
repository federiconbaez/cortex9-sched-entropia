/* Esqueleto. El cuerpo sched_ext está comentado: este kernel no tiene
 * /sys/kernel/sched_ext (sin CONFIG_SCHED_CLASS_EXT).
 * El stub socket sí compila con clang -target bpf y documenta el toolchain.
 */
#include <linux/bpf.h>
#include <bpf/bpf_helpers.h>

char LICENSE[] SEC("license") = "GPL";

/* Mapa que el demonio userspace escribiría (pid -> weight). */
struct {
    __uint(type, BPF_MAP_TYPE_HASH);
    __type(key, __u32);
    __type(value, __u32);
    __uint(max_entries, 4096);
} agent_weight SEC(".maps");

/* Stub cargable: no es el scheduler. Solo prueba toolchain + mapa. */
SEC("socket")
int fnbr_stub(struct __sk_buff *skb)
{
    (void)skb;
    return 0;
}

/*
 * --- sched_ext (NO COMPILAR aquí; requiere headers con SCX_OPS_DEFINE) ---
 *
 * s32 BPF_STRUCT_OPS(entropia_select_cpu, struct task_struct *p, s32 prev_cpu, u64 wake_flags);
 * void BPF_STRUCT_OPS(entropia_enqueue, struct task_struct *p, u64 enq_flags);
 * void BPF_STRUCT_OPS(entropia_dispatch, s32 cpu, struct task_struct *prev);
 * SCX_OPS_DEFINE(entropia_ops, .select_cpu=..., .enqueue=..., .dispatch=..., .name="fnbr_entropia");
 *
 * El select/enqueue leería agent_weight[pid] para ordenar la cola.
 */
