pub struct LocalMounts;

impl LocalMounts {
    pub fn enable() -> Self {
        std::env::set_var("MEMHG_TEST_LOCAL_MOUNTS", "1");
        Self
    }
}

impl Drop for LocalMounts {
    fn drop(&mut self) {
        std::env::remove_var("MEMHG_TEST_LOCAL_MOUNTS");
    }
}
