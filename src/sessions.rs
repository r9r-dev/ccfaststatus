use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

/// Count processes whose exact executable name is `"claude"`.
/// Native equivalent of `ps -Ao comm | grep ^claude$`.
/// Reference JS: getActiveSessions (statusline.mjs:163)
pub fn count() -> usize {
    let mut sys = System::new();
    // Explicitly refresh ONLY what we need (names). The `refresh_processes`
    // convenience enables CPU/mem/disk/exe which are dead work for us.
    sys.refresh_processes_specifics(
        ProcessesToUpdate::All,
        false,
        ProcessRefreshKind::new(),
    );
    sys.processes()
        .values()
        .filter(|p| p.name().to_str() == Some("claude"))
        .count()
}
