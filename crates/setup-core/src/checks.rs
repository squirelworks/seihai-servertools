//! Safety check and the combinations the setup wizard refuses to save.
//!
//! **Today:** the types. **Setup step 2** (`docs/NEXT.md`) fills in
//! [`check_gserver`] and adds the listserver checks. The rules to implement
//! are listed in `docs/design/go-public.md` ("Safety check" and
//! "Combinations the GUI refuses to save"); each rule gets its own test.

use crate::config::options_txt::OptionsTxt;

/// How serious a finding is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Shown for information.
    Info,
    /// Shown; saving is still allowed.
    Warn,
    /// Saving is refused until it is fixed.
    Block,
}

/// One thing the check found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub severity: Severity,
    /// The key the finding is about, if one.
    pub key: Option<String>,
    /// One plain sentence for the owner.
    pub text: String,
}

/// Check a gserver's options. Step 2 adds the rules; today it finds nothing.
pub fn check_gserver(_options: &OptionsTxt) -> Vec<Finding> {
    Vec::new()
}

/// True when nothing in `findings` blocks saving.
pub fn may_save(findings: &[Finding]) -> bool {
    findings.iter().all(|f| f.severity != Severity::Block)
}
