//! Escritura del mapa `agent_weight` del planificador sched_ext `scx_entropia`.
//!
//! El cargador C fija el mapa en bpffs (`/sys/fs/bpf/fnbr_agent_weight`).
//! Acá lo abrimos con `BPF_OBJ_GET` y escribimos `tgid -> peso` con
//! `BPF_MAP_UPDATE_ELEM`, usando la syscall `bpf(2)` directa (sin dependencias).

use std::ffi::CString;
use std::io;
use std::os::raw::{c_int, c_long};

/// Ruta donde `scx/scx_entropia` fija el mapa.
pub const DEFAULT_PIN: &str = "/sys/fs/bpf/fnbr_agent_weight";

const SYS_BPF: c_long = 321; // x86_64
const BPF_MAP_UPDATE_ELEM: c_long = 2;
const BPF_MAP_LOOKUP_ELEM: c_long = 1;
const BPF_OBJ_GET: c_long = 7;
const BPF_ANY: u64 = 0;

extern "C" {
    fn syscall(num: c_long, ...) -> c_long;
    fn close(fd: c_int) -> c_int;
}

/// `union bpf_attr` acolchada a 128 bytes; el kernel exige ceros en lo no usado.
#[repr(C, align(8))]
struct Attr([u8; 128]);

impl Attr {
    fn new() -> Self {
        Attr([0; 128])
    }
    fn put_u32(&mut self, off: usize, v: u32) {
        self.0[off..off + 4].copy_from_slice(&v.to_ne_bytes());
    }
    fn put_u64(&mut self, off: usize, v: u64) {
        self.0[off..off + 8].copy_from_slice(&v.to_ne_bytes());
    }
}

fn bpf(cmd: c_long, attr: &mut Attr) -> io::Result<c_long> {
    // SAFETY: attr apunta a 128 bytes válidos durante la llamada.
    let r = unsafe { syscall(SYS_BPF, cmd, attr as *mut Attr, 128u32) };
    if r < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(r)
    }
}

/// Mapa BPF fijado en bpffs.
pub struct PinnedMap {
    fd: c_int,
}

impl PinnedMap {
    /// Abre el mapa fijado en `path`.
    pub fn open(path: &str) -> io::Result<Self> {
        let c = CString::new(path).map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
        let mut a = Attr::new();
        a.put_u64(0, c.as_ptr() as u64); // pathname
        let fd = bpf(BPF_OBJ_GET, &mut a)?;
        Ok(Self { fd: fd as c_int })
    }

    /// Escribe `tgid -> weight`.
    pub fn set_weight(&self, tgid: u32, weight: u32) -> io::Result<()> {
        let key = tgid;
        let val = weight;
        let mut a = Attr::new();
        a.put_u32(0, self.fd as u32); // map_fd
        a.put_u64(8, &key as *const u32 as u64); // key
        a.put_u64(16, &val as *const u32 as u64); // value
        a.put_u64(24, BPF_ANY); // flags
        bpf(BPF_MAP_UPDATE_ELEM, &mut a).map(|_| ())
    }

    /// Lee el peso de `tgid` (None si no está).
    pub fn get_weight(&self, tgid: u32) -> io::Result<Option<u32>> {
        let key = tgid;
        let mut val: u32 = 0;
        let mut a = Attr::new();
        a.put_u32(0, self.fd as u32);
        a.put_u64(8, &key as *const u32 as u64);
        a.put_u64(16, &mut val as *mut u32 as u64);
        match bpf(BPF_MAP_LOOKUP_ELEM, &mut a) {
            Ok(_) => Ok(Some(val)),
            Err(e) if e.raw_os_error() == Some(2) => Ok(None), // ENOENT
            Err(e) => Err(e),
        }
    }
}

impl Drop for PinnedMap {
    fn drop(&mut self) {
        // SAFETY: fd propio, abierto por BPF_OBJ_GET.
        unsafe {
            close(self.fd);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attr_layout_update_elem() {
        let mut a = Attr::new();
        a.put_u32(0, 7);
        a.put_u64(24, 0xdead_beef);
        assert_eq!(&a.0[0..4], &7u32.to_ne_bytes());
        assert_eq!(&a.0[24..32], &0xdead_beefu64.to_ne_bytes());
        assert!(a.0[32..].iter().all(|&b| b == 0));
    }

    #[test]
    fn open_missing_pin_fails() {
        assert!(PinnedMap::open("/sys/fs/bpf/no-existe-fnbr").is_err());
    }
}
