#!/bin/sh
# Descarga el kernel Ubuntu 6.14 (CONFIG_SCHED_CLASS_EXT=y), extrae su BTF a
# scx/vmlinux.h, compila el struct_ops + cargador + CLI Rust y arma el initramfs.
# Scratch: $VMDIR (por defecto /workspace/fnbr-vm).
set -eu
REPO=$(cd "$(dirname "$0")/.." && pwd)
VMDIR=${VMDIR:-/workspace/fnbr-vm}
DEB=linux-image-unsigned-6.14.0-37-generic_6.14.0-37.37_amd64.deb
SHA=990f51c2440802ddc4c799359c711b40ab5fe4ac9c57b9b4445e5b6264827265
KVER=6.14.0-37-generic
mkdir -p "$VMDIR" && cd "$VMDIR"
[ -f "$DEB" ] || curl -sfO "https://archive.ubuntu.com/ubuntu/pool/main/l/linux/$DEB"
echo "$SHA  $DEB" | sha256sum -c -
[ -f ku/boot/vmlinuz-$KVER ] || { mkdir -p ku && dpkg-deb -x "$DEB" ku; }
chmod 644 ku/boot/vmlinuz-$KVER 2>/dev/null || sudo -n chmod 644 ku/boot/vmlinuz-$KVER

# BTF del kernel invitado -> vmlinux.h (bzImage comprimido con zstd).
if [ ! -f "$REPO/scx/vmlinux.h" ]; then
  python3 - "$VMDIR/ku/boot/vmlinuz-$KVER" "$VMDIR/vmlinux-$KVER" <<'PY'
import sys, io, zstandard
d = open(sys.argv[1], 'rb').read()
off = d.find(b'\x28\xb5\x2f\xfd')
r = zstandard.ZstdDecompressor().stream_reader(io.BytesIO(d[off:]))
with open(sys.argv[2], 'wb') as o:
    while True:
        try: c = r.read(1 << 20)
        except zstandard.ZstdError: break   # basura tras el frame
        if not c: break
        o.write(c)
PY
  /usr/sbin/bpftool btf dump file "$VMDIR/vmlinux-$KVER" format c > "$REPO/scx/vmlinux.h"
fi

cd "$REPO/scx"
clang -O2 -g -target bpf -D__TARGET_ARCH_x86 -I. -c scx_entropia.bpf.c -o scx_entropia.bpf.o
/usr/sbin/bpftool gen skeleton scx_entropia.bpf.o name scx_entropia > scx_entropia.skel.h
clang -O2 -Wall -Wextra -I. scx_entropia.c -o scx_entropia -lbpf -lelf -lz
cd "$REPO" && cargo build --release

# initramfs busybox + binarios + sus bibliotecas dinámicas
R="$VMDIR/rootfs"; rm -rf "$R"
mkdir -p "$R"/bin "$R"/proc "$R"/sys "$R"/dev "$R"/tmp "$R"/root "$R"/lib64 "$R"/lib/x86_64-linux-gnu
cp /usr/bin/busybox "$R/bin/"
cp "$REPO/scx/scx_entropia" "$REPO/target/release/fnbr-sched" "$REPO/vm/bench-guest.sh" "$R/root/"
for b in "$REPO/scx/scx_entropia" "$REPO/target/release/fnbr-sched"; do
  ldd "$b" | awk '/=>/ {print $3} /ld-linux/ {print $1}' | while read -r l; do
    [ -f "$l" ] && cp -L "$l" "$R/lib/x86_64-linux-gnu/"
  done
done
cp -L /lib64/ld-linux-x86-64.so.2 "$R/lib64/"
cat > "$R/init" <<'INIT'
#!/bin/busybox sh
/bin/busybox --install -s /bin
mount -t proc proc /proc; mount -t sysfs sys /sys; mount -t devtmpfs dev /dev
mount -t bpf bpf /sys/fs/bpf; mount -t tmpfs tmpfs /tmp
echo "== uname: $(uname -r)  nproc: $(nproc)"
ls /sys/kernel/sched_ext
T=$(sed -n 's/.*fnbr_T=\([0-9]*\).*/\1/p' /proc/cmdline)
REPS=$(sed -n 's/.*fnbr_REPS=\([0-9]*\).*/\1/p' /proc/cmdline)
T=${T:-8} REPS=${REPS:-3} /root/bench-guest.sh
poweroff -f
INIT
chmod +x "$R/init"
(cd "$R" && find . | cpio -o -H newc 2>/dev/null | gzip > "$VMDIR/initramfs.gz")
echo "initramfs listo: $VMDIR/initramfs.gz"
