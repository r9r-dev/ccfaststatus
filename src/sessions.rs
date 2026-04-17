use sysinfo::{ProcessesToUpdate, System};

/// Count processes whose exact executable name is `"claude"`.
/// Native equivalent of `ps -Ao comm | grep ^claude$`.
/// Reference JS: getActiveSessions (statusline.mjs:163)
pub fn count() -> usize {
    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::All, true);
    sys.processes()
        .values()
        .filter(|p| p.name().to_str() == Some("claude"))
        .count()
}
