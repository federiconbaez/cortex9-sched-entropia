/* SPDX-License-Identifier: GPL-2.0 */
/*
 * scx_entropia — planificador sched_ext (struct_ops) para agentes FNBR.
 *
 * Política: tiempo virtual ponderado (WFQ) sobre una DSQ compartida.
 * El peso de cada tarea sale del mapa `agent_weight` (tgid -> peso 1..10000,
 * escala cgroup: 100 = neutro), que escribe el espacio de usuario en Rust
 * (`fnbr-sched scx-set`) a partir de la entropía del prompt y la urgencia
 * del consenso. Si el tgid no está en el mapa, se usa p->scx.weight (nice).
 *
 * Una tarea con peso w avanza su vtime a razón 100/w por ns ejecutado, así
 * que en contención recibe CPU proporcional a w.
 *
 * Basado en el patrón de scx_simple (sched-ext/scx, kernel tools/sched_ext).
 * Kfuncs de Linux >= 6.13 (scx_bpf_dsq_insert*, scx_bpf_dsq_move_to_local).
 */
#include "vmlinux.h"
#include <bpf/bpf_helpers.h>
#include <bpf/bpf_tracing.h>

char _license[] SEC("license") = "GPL";

#define SHARED_DSQ 0
#define BPF_STRUCT_OPS(name, args...) \
	SEC("struct_ops/" #name) BPF_PROG(name, ##args)
#define BPF_STRUCT_OPS_SLEEPABLE(name, args...) \
	SEC("struct_ops.s/" #name) BPF_PROG(name, ##args)

extern s32 scx_bpf_create_dsq(u64 dsq_id, s32 node) __ksym;
extern s32 scx_bpf_select_cpu_dfl(struct task_struct *p, s32 prev_cpu,
				  u64 wake_flags, bool *is_idle) __ksym;
extern void scx_bpf_dsq_insert(struct task_struct *p, u64 dsq_id, u64 slice,
			       u64 enq_flags) __ksym;
extern void scx_bpf_dsq_insert_vtime(struct task_struct *p, u64 dsq_id,
				     u64 slice, u64 vtime, u64 enq_flags) __ksym;
extern bool scx_bpf_dsq_move_to_local(u64 dsq_id) __ksym;

/* tgid -> peso (escrito por userspace desde el scoring de entropía). */
struct {
	__uint(type, BPF_MAP_TYPE_HASH);
	__type(key, u32);
	__type(value, u32);
	__uint(max_entries, 4096);
} agent_weight SEC(".maps");

/* Estadísticas: 0 = enqueue con peso del mapa, 1 = enqueue sin mapa,
 * 2 = despacho directo a CPU ociosa. */
struct {
	__uint(type, BPF_MAP_TYPE_PERCPU_ARRAY);
	__type(key, u32);
	__type(value, u64);
	__uint(max_entries, 3);
} stats SEC(".maps");

static u64 vtime_now;

static __always_inline void stat_inc(u32 idx)
{
	u64 *c = bpf_map_lookup_elem(&stats, &idx);
	if (c)
		(*c)++;
}

static __always_inline bool vtime_before(u64 a, u64 b)
{
	return (s64)(a - b) < 0;
}

static __always_inline u64 task_weight(struct task_struct *p, bool count)
{
	u32 tgid = p->tgid;
	u32 *w = bpf_map_lookup_elem(&agent_weight, &tgid);

	if (w && *w > 0) {
		if (count)
			stat_inc(0);
		return *w;
	}
	if (count)
		stat_inc(1);
	return p->scx.weight ? p->scx.weight : 100;
}

s32 BPF_STRUCT_OPS(entropia_select_cpu, struct task_struct *p, s32 prev_cpu,
		   u64 wake_flags)
{
	bool is_idle = false;
	s32 cpu = scx_bpf_select_cpu_dfl(p, prev_cpu, wake_flags, &is_idle);

	if (is_idle) {
		stat_inc(2);
		scx_bpf_dsq_insert(p, SCX_DSQ_LOCAL, SCX_SLICE_DFL, 0);
	}
	return cpu;
}

void BPF_STRUCT_OPS(entropia_enqueue, struct task_struct *p, u64 enq_flags)
{
	u64 vtime = p->scx.dsq_vtime;

	task_weight(p, true);
	/* No dejar que una tarea dormida acumule más de un slice de crédito. */
	if (vtime_before(vtime, vtime_now - SCX_SLICE_DFL))
		vtime = vtime_now - SCX_SLICE_DFL;
	scx_bpf_dsq_insert_vtime(p, SHARED_DSQ, SCX_SLICE_DFL, vtime, enq_flags);
}

void BPF_STRUCT_OPS(entropia_dispatch, s32 cpu, struct task_struct *prev)
{
	scx_bpf_dsq_move_to_local(SHARED_DSQ);
}

void BPF_STRUCT_OPS(entropia_running, struct task_struct *p)
{
	if (vtime_before(vtime_now, p->scx.dsq_vtime))
		vtime_now = p->scx.dsq_vtime;
}

void BPF_STRUCT_OPS(entropia_stopping, struct task_struct *p, bool runnable)
{
	u64 used = SCX_SLICE_DFL - p->scx.slice;

	p->scx.dsq_vtime += used * 100 / task_weight(p, false);
}

void BPF_STRUCT_OPS(entropia_enable, struct task_struct *p)
{
	p->scx.dsq_vtime = vtime_now;
}

s32 BPF_STRUCT_OPS_SLEEPABLE(entropia_init)
{
	return scx_bpf_create_dsq(SHARED_DSQ, -1);
}

void BPF_STRUCT_OPS(entropia_exit, struct scx_exit_info *ei)
{
}

SEC(".struct_ops.link")
struct sched_ext_ops entropia_ops = {
	.select_cpu	= (void *)entropia_select_cpu,
	.enqueue	= (void *)entropia_enqueue,
	.dispatch	= (void *)entropia_dispatch,
	.running	= (void *)entropia_running,
	.stopping	= (void *)entropia_stopping,
	.enable		= (void *)entropia_enable,
	.init		= (void *)entropia_init,
	.exit		= (void *)entropia_exit,
	.name		= "fnbr_entropia",
};
