use std::cell::Cell;
use std::sync::Mutex;

thread_local! {
    static FINALIZE_SCAN_LINKS_FAIL: Cell<bool> = const { Cell::new(false) };
    static INDEX_QUEUE_FAIL: Cell<bool> = const { Cell::new(false) };
    static CANCEL_AFTER_PAUSE: Cell<bool> = const { Cell::new(false) };
    static FORCE_INDEX_CANCEL: Cell<bool> = const { Cell::new(false) };
    static FORCE_INVALID_FILE_NAME: Cell<bool> = const { Cell::new(false) };
    static NULL_MTIME: Cell<bool> = const { Cell::new(false) };
    static DISCOVERY_FAIL: Cell<bool> = const { Cell::new(false) };
    static DISCOVERY_PANIC: Cell<bool> = const { Cell::new(false) };
    static INDEX_BATCH_PANIC: Cell<bool> = const { Cell::new(false) };
    static STRIP_PREFIX_FAIL: Cell<bool> = const { Cell::new(false) };
    static STRIP_PREFIX_MAP_ERR: Cell<bool> = const { Cell::new(false) };
    static INDEX_ON_DISK_PANIC: Cell<bool> = const { Cell::new(false) };
    static READ_META_PANIC: Cell<bool> = const { Cell::new(false) };
    static EXPORT_PARALLEL_PANIC: Cell<bool> = const { Cell::new(false) };
}

static ENV_TEST_LOCK: Mutex<()> = Mutex::new(());

fn lock_env_test() -> std::sync::MutexGuard<'static, ()> {
    ENV_TEST_LOCK
        .lock()
        .unwrap_or_else(|error| error.into_inner())
}

pub fn set_finalize_scan_links_fail(active: bool) {
    FINALIZE_SCAN_LINKS_FAIL.set(active);
}

pub fn finalize_scan_links_should_fail() -> bool {
    FINALIZE_SCAN_LINKS_FAIL.get()
}

fn set_thread_flag(name: &str, active: bool) {
    match name {
        "MEMHG_TEST_INDEX_QUEUE_FAIL" => INDEX_QUEUE_FAIL.set(active),
        "MEMHG_TEST_CANCEL_AFTER_PAUSE" => CANCEL_AFTER_PAUSE.set(active),
        "MEMHG_TEST_FORCE_INDEX_CANCEL" => FORCE_INDEX_CANCEL.set(active),
        "MEMHG_TEST_FORCE_INVALID_FILE_NAME" => FORCE_INVALID_FILE_NAME.set(active),
        "MEMHG_TEST_NULL_MTIME" => NULL_MTIME.set(active),
        "MEMHG_TEST_DISCOVERY_FAIL" => DISCOVERY_FAIL.set(active),
        "MEMHG_TEST_DISCOVERY_PANIC" => DISCOVERY_PANIC.set(active),
        "MEMHG_TEST_INDEX_BATCH_PANIC" => INDEX_BATCH_PANIC.set(active),
        "MEMHG_TEST_STRIP_PREFIX_FAIL" => STRIP_PREFIX_FAIL.set(active),
        "MEMHG_TEST_STRIP_PREFIX_MAP_ERR" => STRIP_PREFIX_MAP_ERR.set(active),
        "MEMHG_TEST_INDEX_ON_DISK_PANIC" => INDEX_ON_DISK_PANIC.set(active),
        "MEMHG_TEST_READ_META_PANIC" => READ_META_PANIC.set(active),
        "MEMHG_TEST_EXPORT_PARALLEL_PANIC" => EXPORT_PARALLEL_PANIC.set(active),
        _ => {}
    }
}

pub fn set_flag(name: &str) {
    set_thread_flag(name, true);
}

fn thread_flag_active(name: &str) -> bool {
    match name {
        "MEMHG_TEST_INDEX_QUEUE_FAIL" => INDEX_QUEUE_FAIL.get(),
        "MEMHG_TEST_CANCEL_AFTER_PAUSE" => CANCEL_AFTER_PAUSE.get(),
        "MEMHG_TEST_FORCE_INDEX_CANCEL" => FORCE_INDEX_CANCEL.get(),
        "MEMHG_TEST_FORCE_INVALID_FILE_NAME" => FORCE_INVALID_FILE_NAME.get(),
        "MEMHG_TEST_NULL_MTIME" => NULL_MTIME.get(),
        "MEMHG_TEST_DISCOVERY_FAIL" => DISCOVERY_FAIL.get(),
        "MEMHG_TEST_DISCOVERY_PANIC" => DISCOVERY_PANIC.get(),
        "MEMHG_TEST_INDEX_BATCH_PANIC" => INDEX_BATCH_PANIC.get(),
        "MEMHG_TEST_STRIP_PREFIX_FAIL" => STRIP_PREFIX_FAIL.get(),
        "MEMHG_TEST_STRIP_PREFIX_MAP_ERR" => STRIP_PREFIX_MAP_ERR.get(),
        "MEMHG_TEST_INDEX_ON_DISK_PANIC" => INDEX_ON_DISK_PANIC.get(),
        "MEMHG_TEST_READ_META_PANIC" => READ_META_PANIC.get(),
        "MEMHG_TEST_EXPORT_PARALLEL_PANIC" => EXPORT_PARALLEL_PANIC.get(),
        _ => false,
    }
}

pub fn flag_active(name: &str) -> bool {
    thread_flag_active(name)
}

pub fn take_flag(name: &str) -> bool {
    let active = thread_flag_active(name);
    if active {
        set_thread_flag(name, false);
    }
    active
}

pub fn clear_flag(name: &str) {
    set_thread_flag(name, false);
}

pub(crate) fn reset_unlocked() {
    set_finalize_scan_links_fail(false);
    for name in [
        "MEMHG_TEST_INDEX_QUEUE_FAIL",
        "MEMHG_TEST_CANCEL_AFTER_PAUSE",
        "MEMHG_TEST_FORCE_INDEX_CANCEL",
        "MEMHG_TEST_FORCE_INVALID_FILE_NAME",
        "MEMHG_TEST_NULL_MTIME",
        "MEMHG_TEST_DISCOVERY_FAIL",
        "MEMHG_TEST_DISCOVERY_PANIC",
        "MEMHG_TEST_INDEX_BATCH_PANIC",
        "MEMHG_TEST_STRIP_PREFIX_FAIL",
        "MEMHG_TEST_STRIP_PREFIX_MAP_ERR",
        "MEMHG_TEST_INDEX_ON_DISK_PANIC",
        "MEMHG_TEST_READ_META_PANIC",
        "MEMHG_TEST_EXPORT_PARALLEL_PANIC",
    ] {
        clear_flag(name);
    }
}

pub fn with_env_test_lock<F, R>(f: F) -> R
where
    F: FnOnce() -> R,
{
    let _guard = lock_env_test();
    f()
}

pub fn env_test_guard() -> std::sync::MutexGuard<'static, ()> {
    lock_env_test()
}

pub fn reset() {
    let _guard = lock_env_test();
    reset_unlocked();
}
