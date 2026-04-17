//! Count active Claude Code sessions by scanning running processes for the
//! literal name "claude". On macOS uses libc FFI (no framework deps). On
//! Linux reads /proc. Other platforms return 0 (the segment just omits the
//! sessions count).

/// Count processes whose exec_path basename is "claude".
/// Native equivalent of `ps -Ao comm | grep '^claude$'`.
pub fn count() -> usize {
    imp::count()
}

#[cfg(target_os = "macos")]
mod imp {
    use libc::{c_int, c_void, proc_listpids, sysctl, CTL_KERN, KERN_PROCARGS2};
    use std::mem;

    /// From Apple's `<sys/proc_info.h>`. Not exposed by libc (0.2.185).
    const PROC_ALL_PIDS: u32 = 1;

    /// Size of the KERN_PROCARGS2 per-pid scratch buffer. The kernel returns
    /// argc (c_int) then the NUL-terminated exec_path, then argv, then envp.
    /// The kernel needs a buffer "large enough" to succeed: with tiny
    /// buffers it returns rc=0 but a blob with the exec_path zeroed out
    /// (observed on macOS 15). 8 KiB is a proven sweet spot: fast and always
    /// populates the exec_path bytes.
    const ARGS_BUF_LEN: usize = 8 * 1024;

    pub fn count() -> usize {
        // 1. Probe required pid array size.
        let byte_size = unsafe { proc_listpids(PROC_ALL_PIDS, 0, std::ptr::null_mut(), 0) };
        if byte_size <= 0 {
            return 0;
        }
        let n_pids = (byte_size as usize) / mem::size_of::<c_int>();
        // Slight overallocation to tolerate new processes between calls.
        let mut pids: Vec<c_int> = vec![0; n_pids + 32];
        let capacity_bytes = (pids.len() * mem::size_of::<c_int>()) as c_int;
        let filled_bytes = unsafe {
            proc_listpids(PROC_ALL_PIDS, 0, pids.as_mut_ptr() as *mut _, capacity_bytes)
        };
        if filled_bytes <= 0 {
            return 0;
        }
        let filled = (filled_bytes as usize) / mem::size_of::<c_int>();
        pids.truncate(filled);

        // 2. For each pid, read its exec_path via KERN_PROCARGS2.
        // `ps -Ao comm` reads from the same source (argv[0]/exec_path), which
        // is why the Claude CLI shows up as "claude" even though its resolved
        // binary is a versioned node script at ~/.local/share/claude/versions/.
        // proc_name/proc_pidpath return the resolved binary's basename
        // ("2.1.112"), so they do NOT work for this particular CLI.
        let mut buf = vec![0u8; ARGS_BUF_LEN];
        let mut count = 0;
        for &pid in &pids {
            if pid <= 0 {
                continue;
            }
            if pid_is_claude(pid, &mut buf) {
                count += 1;
            }
        }
        count
    }

    /// Returns true iff `sysctl(KERN_PROCARGS2, pid)` yields an exec_path whose
    /// basename is exactly "claude". Silently false on any error — permission
    /// denied is expected and common for system daemons.
    fn pid_is_claude(pid: c_int, buf: &mut [u8]) -> bool {
        let mut mib: [c_int; 3] = [CTL_KERN, KERN_PROCARGS2, pid];
        let mut len: libc::size_t = buf.len();
        let rc = unsafe {
            sysctl(
                mib.as_mut_ptr(),
                3,
                buf.as_mut_ptr() as *mut c_void,
                &mut len,
                std::ptr::null_mut(),
                0,
            )
        };
        // Layout: [argc: c_int][exec_path\0][argv\0 ...][envp\0 ...]
        let header = mem::size_of::<c_int>();
        if rc != 0 || len < header + 2 {
            return false;
        }
        let data = &buf[header..len];
        let nul = match data.iter().position(|&b| b == 0) {
            Some(p) if p > 0 => p,
            _ => return false, // empty or missing NUL (small-buffer edge case)
        };
        let exec_path = &data[..nul];
        let basename = match exec_path.iter().rposition(|&b| b == b'/') {
            Some(idx) => &exec_path[idx + 1..],
            None => exec_path,
        };
        basename == b"claude"
    }
}

#[cfg(target_os = "linux")]
mod imp {
    use std::fs;

    pub fn count() -> usize {
        let entries = match fs::read_dir("/proc") {
            Ok(e) => e,
            Err(_) => return 0,
        };

        let mut count = 0;
        for entry in entries.flatten() {
            let name = entry.file_name();
            // Skip non-numeric directory names.
            if !name.to_string_lossy().chars().all(|c| c.is_ascii_digit()) {
                continue;
            }
            let mut path = entry.path();
            path.push("comm");
            match fs::read_to_string(&path) {
                Ok(s) if s.trim_end() == "claude" => count += 1,
                _ => {}
            }
        }
        count
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
mod imp {
    pub fn count() -> usize {
        0
    }
}
