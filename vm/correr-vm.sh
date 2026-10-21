#!/bin/sh
# Arranca la VM. Acelerador TCG multihilo (MTTCG): en esta computadora KVM
# anidado provoca "kernel BUG at arch/x86/kvm/x86.c:702" al crear la vCPU.
# Usá ACCEL=kvm en una máquina con KVM sano.
set -eu
VMDIR=${VMDIR:-/workspace/fnbr-vm}
ACCEL=${ACCEL:-tcg,thread=multi}
T=${T:-8}; REPS=${REPS:-3}
exec timeout ${TIMEOUT:-900} qemu-system-x86_64 -accel "$ACCEL" -cpu max -smp 2 -m 1G \
  -kernel "$VMDIR/ku/boot/vmlinuz-6.14.0-37-generic" -initrd "$VMDIR/initramfs.gz" \
  -append "console=ttyS0 quiet panic=-1 fnbr_T=$T fnbr_REPS=$REPS" \
  -display none -serial stdio -no-reboot
