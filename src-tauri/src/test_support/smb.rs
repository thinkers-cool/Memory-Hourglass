use std::cell::Cell;

thread_local! {
    static LOCAL_MOUNTS: Cell<bool> = const { Cell::new(false) };
}

pub fn local_mounts_enabled() -> bool {
    LOCAL_MOUNTS.get()
}

pub fn run_with_local_mounts<R>(enabled: bool, operation: impl FnOnce() -> R) -> R {
    let previous = LOCAL_MOUNTS.get();
    LOCAL_MOUNTS.set(enabled || previous);
    let result = operation();
    LOCAL_MOUNTS.set(previous);
    result
}

pub struct LocalMounts;

impl LocalMounts {
    pub fn enable() -> Self {
        LOCAL_MOUNTS.set(true);
        Self
    }
}

impl Drop for LocalMounts {
    fn drop(&mut self) {
        LOCAL_MOUNTS.set(false);
    }
}
