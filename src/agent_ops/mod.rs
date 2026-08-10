//! Agent-native payload reduction applied to the JSON envelope before it is written.
//!
//! # Why this exists
//!
//! `docsrs-cli` is consumed by LLM agents. Forcing an agent to pipe stdout through
//! `jaq`/`sed` to reach two fields burns tokens on data the agent never wanted. The
//! reduction knobs live in the binary so the cut happens **before** the envelope is
//! handed to the caller.
//!
//! # Fixed application order
//!
//! `filter` → `sort-by` → `dedupe-by` → `max-items` → `select` → `count-only` →
//! `truncate-content` → `max-output-bytes`. The order is a contract, and each step
//! sits where it does for a reason:
//!
//! - sorting **before** deduplication decides *which* duplicate survives, so the
//!   surviving row is the ranked one rather than whichever came first upstream;
//! - limiting **after** deduplication stops a duplicate from consuming a slot the
//!   caller wanted spent on a distinct element;
//! - limiting **before** projection avoids projecting rows that are about to be cut;
//! - `--count-only` therefore counts what survived filtering, deduplication **and**
//!   the limit — never the raw upstream total.
//!
//! Sorting is what makes limiting meaningful: `--max-items` without an order returns
//! an arbitrary slice of the matches, which is a worse answer than the full set.
//!
//! # Result-array location
//!
//! Reduction targets the result array inside `envelope.data`. The array is located by
//! probing [`RESULT_ARRAY_KEYS`] in order, then falling back to the first array of
//! objects on the payload. When `data` is itself an array it is used directly; when no
//! array exists at all, projection applies to the `data` object itself.
//!
//! # Failure envelopes
//!
//! An envelope with `ok: false` is passed through byte-for-byte. `--filter` never
//! silences a failure — a filtered-away error would look like an empty success.
//!
//! # Module layout
//!
//! - [`filter`] — dotted-path addressing and the `--filter` predicate grammar
//! - `reduce` — the envelope transformations (locate, dedupe, project, truncate)
//! - this module — the plan ([`AgentOps`]) and the order in which it is applied
//!
//! # Examples
//!
//! ```
//! use docsrs_cli::agent_ops::AgentOps;
//! use docsrs_cli::cli::AgentArgs;
//!
//! let args = AgentArgs {
//!     select: vec!["name".to_string()],
//!     ..AgentArgs::default()
//! };
//! let ops = AgentOps::from_args(&args).expect("valid knobs");
//! assert!(ops.is_active());
//! ```

pub mod filter;
mod reduce;
#[cfg(test)]
mod tests;

use serde_json::{Map, Value};

use crate::cli::{AgentArgs, Cli};
use crate::error::AppResult;

pub use filter::{Filter, FilterOp};
use reduce::{
    array_mut, dedupe, locate_array_key, project, reconcile_counts, sort_by, truncate_strings,
};

/// Keys probed, in order, when locating the result array inside `envelope.data`.
///
/// Product payloads use `hits` (search) and `checks` / `commands` / `items` (meta
/// commands). `results` and `data` are accepted for forward compatibility.
pub const RESULT_ARRAY_KEYS: &[&str] = &["hits", "results", "items", "checks", "commands", "data"];

/// Sibling integer keys that count the result array and must follow it when it shrinks.
///
/// Deliberately narrow. `emitted` is documented as "hits actually emitted", so it
/// describes *this* envelope's array. `total` is deliberately absent: it counts what
/// the upstream index classified before any limit, and survives reduction unchanged.
pub const EMITTED_COUNT_KEYS: &[&str] = &["emitted"];

/// Resolved set of agent-native reduction knobs for one invocation.
#[derive(Debug, Clone, Default)]
pub struct AgentOps {
    select: Vec<String>,
    filters: Vec<Filter>,
    sort_by: Option<String>,
    dedupe_by: Option<String>,
    max_items: Option<usize>,
    count_only: bool,
    truncate_content: Option<usize>,
    max_output_bytes: Option<u64>,
}

/// Counters reported under `agent_surface` when any knob is active.
#[derive(Debug, Clone, Copy, Default)]
struct Surface {
    input_count: usize,
    output_count: usize,
    limited: bool,
    content_truncated: bool,
    output_truncated: bool,
}

impl AgentOps {
    /// Build the plan from parsed CLI flags.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::ErrorKind::InvalidInput`] when a `--filter` expression
    /// is malformed.
    pub fn from_cli(cli: &Cli) -> AppResult<Self> {
        Self::from_args(&cli.agent)
    }

    /// Build the plan from the reduction flag group.
    ///
    /// `select` accepts CSV or repetition; empty items are dropped so a stray comma
    /// never becomes a projection for the empty key.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::ErrorKind::InvalidInput`] when a `--filter` expression
    /// is malformed.
    pub fn from_args(args: &AgentArgs) -> AppResult<Self> {
        let select: Vec<String> = args
            .select
            .iter()
            .flat_map(|s| s.split(','))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(ToString::to_string)
            .collect();
        let mut filters = Vec::with_capacity(args.filter.len());
        for expr in &args.filter {
            filters.push(Filter::parse(expr)?);
        }
        let non_empty = |s: &Option<String>| s.clone().filter(|v| !v.is_empty());
        Ok(Self {
            select,
            filters,
            sort_by: non_empty(&args.sort_by),
            dedupe_by: non_empty(&args.dedupe_by),
            max_items: args.max_items,
            count_only: args.count_only,
            truncate_content: args.truncate_content,
            max_output_bytes: args.max_output_bytes,
        })
    }

    /// Whether any reduction knob is set.
    ///
    /// `--max-output-bytes` alone does **not** activate the pipeline: per-command
    /// budgeting already enforces it upstream, and activating here would change the
    /// established envelope shape for callers that never asked for `agent_surface`.
    pub fn is_active(&self) -> bool {
        !self.select.is_empty()
            || !self.filters.is_empty()
            || self.sort_by.is_some()
            || self.dedupe_by.is_some()
            || self.max_items.is_some()
            || self.count_only
            || self.truncate_content.is_some()
    }

    /// Apply the pipeline to already-serialized envelope bytes.
    ///
    /// Passes the input through unchanged when it is not a JSON object (shell
    /// completion scripts, human markdown) or when the envelope reports `ok: false`.
    /// The returned buffer always ends with a newline when non-empty.
    pub fn apply_to_bytes(&self, raw: &[u8]) -> Vec<u8> {
        if raw.is_empty() {
            return Vec::new();
        }
        let Ok(mut env) = serde_json::from_slice::<Value>(raw) else {
            return raw.to_vec();
        };
        if !env.is_object() || env.get("ok").and_then(Value::as_bool) == Some(false) {
            return raw.to_vec();
        }
        self.apply_to_envelope(&mut env);
        match serde_json::to_vec(&env) {
            Ok(mut buf) => {
                buf.push(b'\n');
                buf
            }
            Err(_) => raw.to_vec(),
        }
    }

    /// Apply the pipeline in place to a parsed success envelope.
    ///
    /// Adds an `agent_surface` object describing what the reduction did. Callers that
    /// pass an error envelope get an untouched value.
    pub fn apply_to_envelope(&self, env: &mut Value) {
        if env.get("ok").and_then(Value::as_bool) == Some(false) {
            return;
        }
        let mut surface = Surface::default();
        let Some(data) = env.get_mut("data") else {
            return;
        };

        let array_key = locate_array_key(data);
        let strip: Vec<&str> = array_key.as_deref().into_iter().collect();

        if let Some(list) = array_mut(data, array_key.as_deref()) {
            surface.input_count = list.len();
            if !self.filters.is_empty() {
                list.retain(|el| self.filters.iter().all(|f| f.matches(el, &strip)));
            }
            if let Some(key) = &self.sort_by {
                sort_by(list, key, &strip);
            }
            if let Some(key) = &self.dedupe_by {
                dedupe(list, key, &strip);
            }
            if let Some(max) = self.max_items {
                surface.limited = list.len() > max;
                list.truncate(max);
            }
            if !self.select.is_empty() {
                for el in list.iter_mut() {
                    *el = project(el, &self.select, &strip);
                }
            }
            surface.output_count = list.len();
            reconcile_counts(data, surface.output_count);
        } else {
            surface.input_count = 1;
            surface.output_count = 1;
            if !self.select.is_empty() {
                *data = project(data, &self.select, &strip);
            }
        }

        if self.count_only {
            let count = surface.output_count;
            *data = Value::Object(Map::from_iter([("count".to_string(), Value::from(count))]));
        }

        if let Some(limit) = self.truncate_content {
            surface.content_truncated = truncate_strings(data, limit);
        }

        // Attach the surface before budgeting so the byte count measures the envelope
        // the caller actually receives, not a smaller draft of it.
        if let Some(obj) = env.as_object_mut() {
            obj.insert("agent_surface".to_string(), surface.to_value());
        }

        if let Some(max) = self.max_output_bytes {
            surface.output_truncated = self.enforce_output_budget(env, array_key.as_deref(), max);
            if surface.output_truncated {
                if let Some(data) = env.get_mut("data") {
                    if let Some(list) = array_mut(data, array_key.as_deref()) {
                        surface.output_count = list.len();
                    }
                    // The budget dropped rows after the first reconciliation, so the
                    // sibling counter is stale again. Re-running it can only shrink
                    // the envelope (fewer or equal digits), never breach the budget.
                    reconcile_counts(data, surface.output_count);
                }
            }
            if let Some(obj) = env.as_object_mut() {
                obj.insert("agent_surface".to_string(), surface.to_value());
            }
        }
    }

    /// Drop trailing elements until the serialized envelope fits `max` bytes.
    ///
    /// Never slices the JSON text: a mid-string cut would emit an unparseable
    /// document, which is strictly worse than an over-budget one.
    fn enforce_output_budget(&self, env: &mut Value, array_key: Option<&str>, max: u64) -> bool {
        let mut dropped = false;
        loop {
            let len = match serde_json::to_vec(env) {
                Ok(buf) => buf.len() as u64,
                Err(_) => return dropped,
            };
            if len <= max {
                return dropped;
            }
            let Some(data) = env.get_mut("data") else {
                return dropped;
            };
            let Some(list) = array_mut(data, array_key) else {
                return dropped;
            };
            if list.pop().is_none() {
                return true;
            }
            dropped = true;
        }
    }
}

impl Surface {
    fn to_value(self) -> Value {
        Value::Object(Map::from_iter([
            ("input_count".to_string(), Value::from(self.input_count)),
            ("output_count".to_string(), Value::from(self.output_count)),
            // Distinguishes "the set was small" from "the set was cut": without it a
            // caller cannot tell whether raising --max-items would return more.
            ("limited".to_string(), Value::Bool(self.limited)),
            (
                "content_truncated".to_string(),
                Value::Bool(self.content_truncated),
            ),
            (
                "output_truncated".to_string(),
                Value::Bool(self.output_truncated),
            ),
        ]))
    }
}
