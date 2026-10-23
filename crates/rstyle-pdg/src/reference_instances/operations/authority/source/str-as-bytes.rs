// core/src/str/mod.rs:513; inherent str method. The operation copies the shared
// string pointer/byte length into a shared byte-slice view of the same storage.
pub const fn as_bytes(&self) -> &[u8] {
    // SAFETY: const sound because we transmute two types with the same layout
    unsafe { mem::transmute(self) }
}
