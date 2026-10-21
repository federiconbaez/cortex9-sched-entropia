#!/usr/bin/env python3
"""Resume un log de vm/correr-vm.sh: cuota de CPU por agente vs peso esperado."""
import re, sys, statistics as st
W = {"A": 846, "B": 514, "C": 182, "D": 50}
tot_w = sum(W.values())
runs = {"base": [], "scx": []}
for line in open(sys.argv[1], encoding="utf-8", errors="replace"):
    m = re.match(r"RESULT (base|scx) rep=\d+ A=(\d+) B=(\d+) C=(\d+) D=(\d+)", line)
    if m:
        runs[m.group(1)].append(dict(zip("ABCD", map(int, m.groups()[1:]))))
print(f"reps: base={len(runs['base'])} scx={len(runs['scx'])}")
for fase, rs in runs.items():
    print(f"\n[{fase}]  agente  peso  cuota_esperada  cuota_medida(media±sd)")
    for k in "ABCD":
        sh = [r[k] / sum(r.values()) for r in rs]
        esp = W[k] / tot_w if fase == "scx" else 0.25
        print(f"         {k}     {W[k]:4d}   {esp*100:6.2f}%        {st.mean(sh)*100:6.2f}% ± {st.pstdev(sh)*100:.2f}")
    rat = [r["A"] / max(r["D"], 1) for r in rs]
    print(f"   ratio A/D (alta/baja prioridad): {st.mean(rat):.2f} (min {min(rat):.2f}, max {max(rat):.2f}); esperado {W['A']/W['D']:.2f} en scx, 1.00 en base")
