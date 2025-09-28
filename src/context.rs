/// Global evaluation context and options.
/// For now, it is intentionally small (CmpMode removed as requested).
#[derive(Clone, Default)]
pub struct Context {
    /// Reserved for future knobs like case sensitivity or feature flags.
    pub(crate) _reserved: (),
}