#!/bin/sh
# Corre DENTRO de la VM (initramfs busybox). Compara CPU recibida por 4
# "agentes" que compiten en la CPU 1: (A) planificador por defecto del kernel
# (EEVDF, todos nice 0) vs (B) sched_ext scx_entropia con pesos del scoring.
T=${T:-8}; REPS=${REPS:-3}; CPU=${CPU:-1}
PA="Consenso epistémico urgente: ¿la anomalía del grafo MCP 0x7f3a es inyección? Δ=3.14 ∑σ²; votos {α:0.91, β:0.12} #FNBR!"
PB="resumir el registro del dia y anotar los cambios del repositorio"
PC="ok ok ok ok ok ok ok ok ok ok ok ok ok ok ok ok"
PD="aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
UA=0.95; UB=0.50; UC=0.10; UD=0.00

ticks() { awk '{print $14+$15}' /proc/$1/stat; }
hog() { taskset -c $CPU sh -c 'while :; do :; done' </dev/null >/dev/null 2>&1 & echo $!; }

medir() { # $1=etiqueta ; usa $A $B $C $D
  a0=$(ticks $A); b0=$(ticks $B); c0=$(ticks $C); d0=$(ticks $D)
  sleep $T
  a1=$(ticks $A); b1=$(ticks $B); c1=$(ticks $C); d1=$(ticks $D)
  echo "RESULT $1 A=$((a1-a0)) B=$((b1-b0)) C=$((c1-c0)) D=$((d1-d0))"
}

echo "== sched_ext state: $(cat /sys/kernel/sched_ext/state)"
for r in $(seq 1 $REPS); do
  # --- Fase A: planificador por defecto ---
  A=$(hog); B=$(hog); C=$(hog); D=$(hog); sleep 1
  echo "== rep $r fase A (sin sched_ext) ops=$(cat /sys/kernel/sched_ext/root/ops 2>/dev/null)"
  medir "base rep=$r"
  kill $A $B $C $D; wait $A $B $C $D 2>/dev/null

  # --- Fase B: sched_ext scx_entropia ---
  rm -f /tmp/scx_stop
  /root/scx_entropia /tmp/scx_stop > /tmp/scx.log 2>&1 &
  L=$!
  i=0; while ! grep -q adjuntado /tmp/scx.log && [ $i -lt 100 ]; do sleep 0.1; i=$((i+1)); done
  cat /tmp/scx.log
  echo "== state=$(cat /sys/kernel/sched_ext/state) ops=$(cat /sys/kernel/sched_ext/root/ops) enable_seq=$(cat /sys/kernel/sched_ext/enable_seq)"
  A=$(hog); B=$(hog); C=$(hog); D=$(hog)
  /root/fnbr-sched scx-set --pid $A --prompt "$PA" --urgency $UA
  /root/fnbr-sched scx-set --pid $B --prompt "$PB" --urgency $UB
  /root/fnbr-sched scx-set --pid $C --prompt "$PC" --urgency $UC
  /root/fnbr-sched scx-set --pid $D --prompt "$PD" --urgency $UD
  sleep 1
  medir "scx rep=$r"
  kill $A $B $C $D; wait $A $B $C $D 2>/dev/null
  touch /tmp/scx_stop; wait $L
  cat /tmp/scx.log | tail -2
  echo "== tras desadjuntar: state=$(cat /sys/kernel/sched_ext/state)"
done
dmesg | grep -i sched_ext | tail -5
echo "== FIN"
