//! Product policy gates, run by `cargo test --test policy_gates`.
//!
//! These replace `scripts/check-policy.sh`, which was 540 lines of bash wrapping
//! 260 lines of inline Python. Two things were wrong with that, one of them
//! silent for months:
//!
//! 1. The product rule is full-stack Rust, and the file that enforced product
//!    rules was written in Python. Three audits missed it because every audit
//!    pointed at `src/`, never at `scripts/` — the verifier is the one thing
//!    nobody points the verifier at.
//!
//! 2. The crate promises Linux, macOS and Windows, and its policy was verifiable
//!    only where `bash`, `python3`, `fd` and `rg` all happened to exist. A
//!    Windows maintainer could not run a single gate.
//!
//! As integration tests they run inside `cargo test`, on every platform, with no
//! interpreter and no external CLI.
//!
//! Each module keeps the rule that made the shell version worth porting: the
//! expectation is derived from the tree, never from a list kept beside it.

mod policy;
