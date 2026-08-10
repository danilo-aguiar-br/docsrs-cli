//! Unit tests for the rustdoc associated-item anchor families.

use super::super::assoc::*;
use crate::item_kind::ItemKind;

fn segs(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|s| (*s).to_string()).collect()
}

#[test]
fn every_family_declares_a_primary_prefix() {
    for family in [
        AssocAnchorKind::Method,
        AssocAnchorKind::Type,
        AssocAnchorKind::Constant,
    ] {
        let prefixes = family.anchor_prefixes();
        assert!(!prefixes.is_empty(), "{family:?} declares no prefix");
        assert_eq!(family.primary_prefix(), prefixes[0], "{family:?}");
        assert!(
            prefixes.iter().all(|p| p.ends_with('.')),
            "{family:?} prefixes={prefixes:?}"
        );
    }
}

#[test]
fn method_family_probes_both_flavours_in_order() {
    // Probe order, never document order, decides the winner when a trait page
    // carries `method.X` and `tymethod.X` for the same name.
    assert_eq!(
        AssocAnchorKind::Method.anchor_prefixes(),
        &["method.", "tymethod."]
    );
    assert_eq!(
        AssocAnchorKind::Type.anchor_prefixes(),
        &["associatedtype."]
    );
    assert_eq!(
        AssocAnchorKind::Constant.anchor_prefixes(),
        &["associatedconstant."]
    );
}

#[test]
fn variant_and_field_route_through_their_own_families() {
    // GAP-ASSOCITEM-002: `Option::Some` and `Range::start` are members like any
    // other, so they belong on the family axis rather than in a special case.
    assert_eq!(
        associated_item_path(ItemKind::Variant, &segs(&["Option", "Some"])),
        Some(AssocAnchorKind::Variant)
    );
    assert_eq!(
        associated_item_path(ItemKind::StructField, &segs(&["Range", "start"])),
        Some(AssocAnchorKind::StructField)
    );
    assert_eq!(AssocAnchorKind::Variant.primary_prefix(), "variant.");
    assert_eq!(
        AssocAnchorKind::StructField.primary_prefix(),
        "structfield."
    );
    // A variant can only live on an enum; probing anything else would burn a
    // rate-limited GET on a page that cannot host the anchor.
    assert_eq!(
        AssocAnchorKind::Variant.parent_kind_probe(),
        &[ItemKind::Enum]
    );
    // Rustdoc spells union fields with the same prefix as struct fields.
    assert_eq!(
        AssocAnchorKind::StructField.parent_kind_probe(),
        &[ItemKind::Struct, ItemKind::Union]
    );
}

#[test]
fn member_only_kinds_are_flagged_regardless_of_path_shape() {
    // Unqualified `variant Some` must be rejectable *before* URL building, since
    // the free-item branch would happily emit `variant.Some.html`.
    assert_eq!(
        member_only_family(ItemKind::Variant),
        Some(AssocAnchorKind::Variant)
    );
    assert_eq!(
        member_only_family(ItemKind::StructField),
        Some(AssocAnchorKind::StructField)
    );
    // Kinds with a page of their own are never member-only.
    for k in [
        ItemKind::Fn,
        ItemKind::Type,
        ItemKind::Constant,
        ItemKind::Struct,
    ] {
        assert_eq!(member_only_family(k), None, "{k:?}");
    }
}

#[test]
fn fragment_recovers_the_two_new_families() {
    assert_eq!(
        assoc_from_fragment("variant.Some"),
        Some((AssocAnchorKind::Variant, "Some"))
    );
    assert_eq!(
        assoc_from_fragment("structfield.start"),
        Some((AssocAnchorKind::StructField, "start"))
    );
}

#[test]
fn dry_run_names_are_the_live_probe_order_verbatim() {
    // This used to compare two hand-kept tables that could drift, which is how a
    // kind goes missing from the dry-run plan while staying reachable in the
    // live probe. The names are now derived from the kind list, so drift is
    // unrepresentable and the assertion below covers what is left: the wire
    // spelling of every probed kind, in probe order, with nothing dropped.
    for family in [
        AssocAnchorKind::Method,
        AssocAnchorKind::Type,
        AssocAnchorKind::Constant,
    ] {
        let names: Vec<&str> = family.parent_kind_probe_names().collect();
        assert_eq!(
            names.len(),
            family.parent_kind_probe().len(),
            "{family:?} dropped a kind"
        );
        assert!(!names.is_empty(), "{family:?} has no parent to probe");
        for (name, kind) in names.iter().zip(family.parent_kind_probe()) {
            assert_eq!(*name, kind.as_str(), "{family:?} order broke at {name}");
        }
    }
}

#[test]
fn probe_order_leads_with_the_kind_that_hosts_the_family() {
    // Associated types live on traits; probing struct first burned two GETs
    // against a rate-limited origin on every lookup.
    assert_eq!(
        AssocAnchorKind::Type.parent_kind_probe()[0],
        ItemKind::Trait
    );
    assert_eq!(
        AssocAnchorKind::Method.parent_kind_probe()[0],
        ItemKind::Struct
    );
    assert_eq!(
        AssocAnchorKind::Constant.parent_kind_probe()[0],
        ItemKind::Struct
    );
    // Every family must still be able to reach every parent kind.
    for family in [
        AssocAnchorKind::Method,
        AssocAnchorKind::Type,
        AssocAnchorKind::Constant,
    ] {
        assert_eq!(family.parent_kind_probe().len(), 5, "{family:?}");
    }
}

#[test]
fn detects_each_family_from_kind_and_leaf_case() {
    assert_eq!(
        associated_item_path(ItemKind::Fn, &segs(&["Runtime", "new"])),
        Some(AssocAnchorKind::Method)
    );
    assert_eq!(
        associated_item_path(ItemKind::Type, &segs(&["Iterator", "Item"])),
        Some(AssocAnchorKind::Type)
    );
    assert_eq!(
        associated_item_path(ItemKind::Constant, &segs(&["Duration", "MAX"])),
        Some(AssocAnchorKind::Constant)
    );
}

#[test]
fn lowercase_parent_keeps_free_item_path() {
    // REGRESSION GUARD: `u32::MAX` and `f32::EPSILON` are associated constants
    // on primitives, but the standard library still serves them from the legacy
    // module page. Routing them through the parent-page anchor would break two
    // lookups that work today. Rust modules and primitives are lowercase.
    assert_eq!(
        associated_item_path(ItemKind::Constant, &segs(&["u32", "MAX"])),
        None
    );
    assert_eq!(
        associated_item_path(ItemKind::Constant, &segs(&["f32", "EPSILON"])),
        None
    );
    // Same rule keeps a module-qualified free type alias off the anchor path.
    assert_eq!(
        associated_item_path(ItemKind::Type, &segs(&["io", "Result"])),
        None
    );
}

#[test]
fn rejects_shapes_outside_the_family() {
    // Single segment: there is no parent to anchor against.
    assert_eq!(associated_item_path(ItemKind::Fn, &segs(&["spawn"])), None);
    // Kinds that own a page of their own are never members.
    assert_eq!(
        associated_item_path(ItemKind::Struct, &segs(&["Runtime", "Builder"])),
        None
    );
    assert_eq!(
        associated_item_path(ItemKind::Static, &segs(&["Thing", "VALUE"])),
        None
    );
    // An empty leaf (`Parent::`) has no member name to anchor.
    assert_eq!(
        associated_item_path(ItemKind::Type, &segs(&["Iterator", ""])),
        None
    );
}

#[test]
fn case_mismatch_still_routes_to_parent_anchor() {
    // Leaf letter case is a human naming convention, never a routing
    // discriminator. Once the parent is known to be a type, the member lives on
    // the parent page whatever the leaf looks like.
    //
    // Guarding on leaf case used to send these three to the free-item branch,
    // which built paths no rustdoc ever emitted
    // (`std/iter/Iterator/type.item.html`) and returned no suggestions. Routing
    // them to the parent page turns an impossible URL into a not-found that can
    // name the real members.
    assert_eq!(
        associated_item_path(ItemKind::Type, &segs(&["Iterator", "item"])),
        Some(AssocAnchorKind::Type)
    );
    assert_eq!(
        associated_item_path(ItemKind::Constant, &segs(&["Duration", "max"])),
        Some(AssocAnchorKind::Constant)
    );
    assert_eq!(
        associated_item_path(ItemKind::Fn, &segs(&["Iterator", "Item"])),
        Some(AssocAnchorKind::Method)
    );
}

#[test]
fn fragment_names_its_own_family() {
    // The fetch path reads the family back off the URL it already holds instead
    // of threading an extra parameter through every layer.
    assert_eq!(
        assoc_from_fragment("tymethod.next"),
        Some((AssocAnchorKind::Method, "next"))
    );
    assert_eq!(
        assoc_from_fragment("method.map"),
        Some((AssocAnchorKind::Method, "map"))
    );
    assert_eq!(
        assoc_from_fragment("associatedtype.Item"),
        Some((AssocAnchorKind::Type, "Item"))
    );
    assert_eq!(
        assoc_from_fragment("associatedconstant.MAX"),
        Some((AssocAnchorKind::Constant, "MAX"))
    );
    // `variant.Some` used to be this test's example of "outside the family",
    // which quietly made the suite the guardian of the missing support
    // (GAP-ASSOCITEM-002). Section headings are the honest counter-example:
    // rustdoc emits them as ids too, and they name no member at all.
    assert_eq!(assoc_from_fragment("required-methods"), None);
    assert_eq!(assoc_from_fragment("implementations"), None);
    // A bare prefix carries no member name.
    assert_eq!(assoc_from_fragment("associatedtype."), None);
    assert_eq!(assoc_from_fragment("variant."), None);
}

#[test]
fn anchor_ids_follow_probe_order() {
    assert_eq!(
        AssocAnchorKind::Method.anchor_ids("next"),
        vec!["method.next".to_string(), "tymethod.next".to_string()]
    );
    assert_eq!(
        AssocAnchorKind::Type.anchor_ids("Item"),
        vec!["associatedtype.Item".to_string()]
    );
}

#[test]
fn strip_rejects_empty_and_foreign_ids() {
    let prefixes = AssocAnchorKind::Type.anchor_prefixes();
    assert_eq!(
        strip_assoc_anchor_prefix("associatedtype.Item", prefixes),
        Some("Item")
    );
    assert_eq!(strip_assoc_anchor_prefix("associatedtype.", prefixes), None);
    assert_eq!(strip_assoc_anchor_prefix("method.next", prefixes), None);
}
