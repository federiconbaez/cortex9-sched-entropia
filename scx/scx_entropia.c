// SPDX-License-Identifier: GPL-2.0
/*
 * Cargador de scx_entropia: carga el struct_ops sched_ext, fija el mapa
 * agent_weight en bpffs para que `fnbr-sched scx-set` (Rust) lo escriba,
 * y queda corriendo hasta SIGINT/SIGTERM o hasta que exista el archivo stop.
 */
#include <errno.h>
#include <signal.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>
#include <bpf/bpf.h>
#include <bpf/libbpf.h>
#include "scx_entropia.skel.h"

#define PIN_PATH "/sys/fs/bpf/fnbr_agent_weight"

static volatile sig_atomic_t stop;
static void on_sig(int s) { (void)s; stop = 1; }

static void read_stats(struct scx_entropia *skel, unsigned long long out[3])
{
	int ncpu = libbpf_num_possible_cpus();
	unsigned long long vals[ncpu];
	for (unsigned int k = 0; k < 3; k++) {
		out[k] = 0;
		if (bpf_map__lookup_elem(skel->maps.stats, &k, sizeof(k), vals,
					 sizeof(vals), 0) == 0)
			for (int c = 0; c < ncpu; c++)
				out[k] += vals[c];
	}
}

int main(int argc, char **argv)
{
	const char *stopfile = argc > 1 ? argv[1] : "/tmp/scx_stop";
	struct scx_entropia *skel;
	struct bpf_link *link;
	unsigned long long st[3];

	signal(SIGINT, on_sig);
	signal(SIGTERM, on_sig);
	libbpf_set_strict_mode(LIBBPF_STRICT_ALL);

	skel = scx_entropia__open_and_load();
	if (!skel) {
		fprintf(stderr, "scx_entropia: open/load falló: %s\n", strerror(errno));
		return 1;
	}
	unlink(PIN_PATH);
	if (bpf_map__pin(skel->maps.agent_weight, PIN_PATH)) {
		fprintf(stderr, "scx_entropia: pin falló: %s\n", strerror(errno));
		return 1;
	}
	link = bpf_map__attach_struct_ops(skel->maps.entropia_ops);
	if (!link) {
		fprintf(stderr, "scx_entropia: attach struct_ops falló: %s\n", strerror(errno));
		bpf_map__unpin(skel->maps.agent_weight, PIN_PATH);
		return 1;
	}
	printf("scx_entropia: struct_ops 'fnbr_entropia' adjuntado; mapa en %s\n", PIN_PATH);
	fflush(stdout);

	while (!stop && access(stopfile, F_OK) != 0)
		usleep(100000);

	read_stats(skel, st);
	printf("scx_entropia: stats enqueue_con_peso=%llu enqueue_sin_mapa=%llu directo_ocioso=%llu\n",
	       st[0], st[1], st[2]);
	bpf_link__destroy(link);
	bpf_map__unpin(skel->maps.agent_weight, PIN_PATH);
	scx_entropia__destroy(skel);
	printf("scx_entropia: desadjuntado\n");
	return 0;
}
