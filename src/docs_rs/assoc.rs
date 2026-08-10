//! Rustdoc associated-item anchor families.
//!
//! Rustdoc does not have "the method anchor". It emits **one id prefix per
//! member category**, and every category coexists inside the parent page:
//!
//! | Category                 | Anchor id prefix       | Parent page          |
//! |--------------------------|------------------------|----------------------|
//! | inherent / default method| `method.`              | struct, enum, trait… |
//! | required trait method    | `tymethod.`            | trait                |
//! | associated type          | `associatedtype.`      | trait                |
//! | associated constant      | `associatedconstant.`  | struct, trait        |
//! | enum variant             | `variant.`             | enum                 |
//! | struct / union field     | `structfield.`         | struct, union        |
//!
//! Measured on `std::iter::Iterator`: 198 `associatedtype.` ids, 75 `method.`
//! ids and 1 `tymethod.` id on a single page.
//!
//! Modelling this as "two spellings of method" is what let `Iterator::Item` and
//! `Duration::MAX` build impossible URLs (`std/iter/Iterator/type.Item.html`)
//! long after required trait methods were fixed. The axis is the **category**,
//! so this module owns the whole family in one place: prefixes, parent-page
//! probe order, and the detection rule that decides whether a path is an
//! associated item at all.
//!
//! Pure data and pure functions — no HTTP, no HTML parse (one-shot safe).

use crate::item_kind::ItemKind;

/// Anchor id prefixes for an associated function, in probe order.
///
/// Both can coexist on one trait page, so probe order — never document order —
/// decides the winner. The split is upstream behaviour: rustc keeps
/// `tests/rustdoc-html/anchors/anchor-id-trait-tymethod-28478.rs` as a guard.
pub const METHOD_ANCHOR_PREFIXES: &[&str] = &["method.", "tymethod."];

/// Anchor id prefix for an associated type (`type Item;` inside a trait).
const TYPE_ANCHOR_PREFIXES: &[&str] = &["associatedtype."];

/// Anchor id prefix for an associated constant (`const MAX: Self;`).
const CONSTANT_ANCHOR_PREFIXES: &[&str] = &["associatedconstant."];

/// Anchor id prefix for an enum variant (`Option::Some`).
const VARIANT_ANCHOR_PREFIXES: &[&str] = &["variant."];

/// Anchor id prefix for a struct or union field (`Range::start`).
const STRUCT_FIELD_ANCHOR_PREFIXES: &[&str] = &["structfield."];

/// Sentinel prefix of the "anchor absent on the parent page" error message.
///
/// The live probe in [`crate::docs_rs`] and the recovery path in
/// `crate::ops` both match on it to stop walking parent kinds once the parent
/// page was found but the member was not. It is a contract between those call
/// sites, so it lives here instead of being retyped as a literal at each one.
pub const ASSOC_ANCHOR_MISS_PREFIX: &str = "associated item anchor ";

/// Parent kinds probed for an associated function (struct first — common case).
const METHOD_PARENT_KINDS: &[ItemKind] = &[
    ItemKind::Struct,
    ItemKind::Enum,
    ItemKind::Trait,
    ItemKind::Type,
    ItemKind::Union,
];

/// Parent kinds probed for an associated type (trait first — they live there).
const TYPE_PARENT_KINDS: &[ItemKind] = &[
    ItemKind::Trait,
    ItemKind::Struct,
    ItemKind::Enum,
    ItemKind::Union,
    ItemKind::Type,
];

/// Parent kinds probed for an associated constant (struct first: `Duration::MAX`).
const CONSTANT_PARENT_KINDS: &[ItemKind] = &[
    ItemKind::Struct,
    ItemKind::Trait,
    ItemKind::Enum,
    ItemKind::Union,
    ItemKind::Type,
];

/// Parent kinds probed for an enum variant.
///
/// Exactly one entry: only an `enum` has variants, so probing anything else
/// would spend a rate-limited GET on a page that cannot host the anchor.
const VARIANT_PARENT_KINDS: &[ItemKind] = &[ItemKind::Enum];

/// Parent kinds probed for a field: structs first, then unions.
///
/// Rustdoc uses the same `structfield.` prefix for both, so the family is one
/// axis with two possible hosts.
const STRUCT_FIELD_PARENT_KINDS: &[ItemKind] = &[ItemKind::Struct, ItemKind::Union];

/// Which rustdoc anchor family a `Parent::member` path resolves through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssocAnchorKind {
    /// Inherent method, defaulted trait method, or required trait method.
    Method,
    /// Associated type declared inside a trait.
    Type,
    /// Associated constant declared on a type or trait.
    Constant,
    /// Enum variant (`Option::Some`).
    Variant,
    /// Struct or union field (`Range::start`).
    StructField,
}

impl AssocAnchorKind {
    /// Anchor id prefixes to try, in probe order.
    pub fn anchor_prefixes(self) -> &'static [&'static str] {
        match self {
            Self::Method => METHOD_ANCHOR_PREFIXES,
            Self::Type => TYPE_ANCHOR_PREFIXES,
            Self::Constant => CONSTANT_ANCHOR_PREFIXES,
            Self::Variant => VARIANT_ANCHOR_PREFIXES,
            Self::StructField => STRUCT_FIELD_ANCHOR_PREFIXES,
        }
    }

    /// Prefix the URL builder bakes into the planned fragment.
    ///
    /// Only one fragment fits in a URL; the extractor still tries every prefix
    /// of the family against the fetched page.
    pub fn primary_prefix(self) -> &'static str {
        // Every family declares at least one prefix, so index 0 always exists.
        self.anchor_prefixes()[0]
    }

    /// Parent page kinds to probe, in order.
    ///
    /// Single source of truth: the live probe in [`crate::docs_rs`] and the
    /// suggestion probe in `crate::ops` must walk the same kinds in the same
    /// order, or a member found by one stays invisible to the other.
    pub fn parent_kind_probe(self) -> &'static [ItemKind] {
        match self {
            Self::Method => METHOD_PARENT_KINDS,
            Self::Type => TYPE_PARENT_KINDS,
            Self::Constant => CONSTANT_PARENT_KINDS,
            Self::Variant => VARIANT_PARENT_KINDS,
            Self::StructField => STRUCT_FIELD_PARENT_KINDS,
        }
    }

    /// [`Self::parent_kind_probe`] as wire strings for the `--dry-run` envelope.
    ///
    /// Derived from the kind list rather than mirrored by hand. Hand-kept string
    /// mirrors used to sit right below the "single source of truth" promise
    /// above, and they made `--dry-run` capable of *lying*: reordering the kinds
    /// without reordering the strings would have published a probe order the
    /// live fetch never walks, to an agent that plans on exactly that field.
    pub fn parent_kind_probe_names(self) -> impl Iterator<Item = &'static str> + Clone {
        self.parent_kind_probe().iter().map(|k| k.as_str())
    }

    /// Build the ordered anchor ids a live fetch will try for `name`.
    pub fn anchor_ids(self, name: &str) -> Vec<String> {
        self.anchor_prefixes()
            .iter()
            .map(|prefix| format!("{prefix}{name}"))
            .collect()
    }
}

/// Detect a parent-qualified associated item: `Parent::member`.
///
/// `segs` must already have the crate-root prefix stripped (see
/// [`super::strip_crate_prefix_segments`]).
///
/// Exactly **one** predicate decides this: the parent segment starts with an
/// ASCII uppercase letter. An uppercase module would fire rustc's
/// `non_snake_case` lint, so an uppercase parent *is* a type — never a module —
/// and the path therefore can never be a free item. That single rule keeps
/// `u32::MAX` and `f32::EPSILON` on the legacy free-constant page
/// (`std/u32/constant.MAX.html`) the standard library still serves, while
/// routing `Duration::MAX` to `struct.Duration.html#associatedconstant.MAX`.
///
/// The leaf's letter case is **not** consulted. It looks like a second
/// discriminator, but it adds no discriminating power once the parent is known
/// to be a type — it only narrows the domain, and rebuilds the original defect
/// inside the range it excludes. With a leaf-case guard, `Iterator::item` fell
/// through to the free-item branch and built `std/iter/Iterator/type.item.html`,
/// a path no rustdoc ever emitted, with no suggestions. Without it, the same
/// typo reaches the parent page and fails with the real member names.
///
/// Returns `None` for every other shape, so the caller falls back to the free
/// item page (`{kind}.{name}.html`).
pub fn associated_item_path(kind: ItemKind, segs: &[String]) -> Option<AssocAnchorKind> {
    let [.., parent, leaf] = segs else {
        return None;
    };
    if !parent
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_uppercase())
    {
        return None;
    }
    // An empty leaf (`Parent::`) would build an anchor with no member name.
    if leaf.is_empty() {
        return None;
    }
    match kind {
        ItemKind::Fn => Some(AssocAnchorKind::Method),
        ItemKind::Type => Some(AssocAnchorKind::Type),
        ItemKind::Constant => Some(AssocAnchorKind::Constant),
        ItemKind::Variant => Some(AssocAnchorKind::Variant),
        ItemKind::StructField => Some(AssocAnchorKind::StructField),
        // Kinds that own a page of their own are never members of a parent.
        _ => None,
    }
}

/// Anchor family a kind resolves through *regardless* of path shape.
///
/// Returns `Some` only for kinds that rustdoc never gives a page of their own,
/// so a caller can reject `variant Some` (no parent) with the real reason
/// instead of letting it fall through and build `variant.Some.html`.
pub fn member_only_family(kind: ItemKind) -> Option<AssocAnchorKind> {
    match kind {
        ItemKind::Variant => Some(AssocAnchorKind::Variant),
        ItemKind::StructField => Some(AssocAnchorKind::StructField),
        _ => None,
    }
}

/// Recover the anchor family and bare member name from a planned URL fragment.
///
/// A fragment names its own family without ambiguity, so the fetch path reads
/// the family off the URL it already holds instead of threading an extra
/// parameter through every layer.
pub fn assoc_from_fragment(fragment: &str) -> Option<(AssocAnchorKind, &str)> {
    const FAMILIES: &[AssocAnchorKind] = &[
        AssocAnchorKind::Method,
        AssocAnchorKind::Type,
        AssocAnchorKind::Constant,
        AssocAnchorKind::Variant,
        AssocAnchorKind::StructField,
    ];
    FAMILIES.iter().find_map(|family| {
        family
            .anchor_prefixes()
            .iter()
            .find_map(|prefix| fragment.strip_prefix(prefix))
            .filter(|name| !name.is_empty())
            .map(|name| (*family, name))
    })
}

/// Strip any prefix of `prefixes` from a rustdoc anchor id.
///
/// Returns the bare member name, or `None` when the id names something outside
/// the family (`variant.Some`, a section heading, …).
pub fn strip_assoc_anchor_prefix<'a>(id: &'a str, prefixes: &[&str]) -> Option<&'a str> {
    prefixes
        .iter()
        .find_map(|prefix| id.strip_prefix(prefix))
        .filter(|name| !name.is_empty())
}
