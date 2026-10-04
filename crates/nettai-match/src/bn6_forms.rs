//! BN6's forms say of each other what BN6's systems mean
//! (docs/design/rules-in-luau.md, S7b): the forms system's kinds and games,
//! a Cross's navi and form in Beast Out, the navis' form sets, and the
//! framework's behavior traits that follow from the kinds (the original's
//! tests of the form's number). The engine checks only the extensions'
//! types; these are BN6's own rules.

use crate::testing::bn6_content;
use bn6_compat::forms::{self, Kind};
use nettai_battle::content::FormTraits;
use nettai_content_api::FormHandle;

#[test]
fn bn6_forms_agree_with_their_kinds() {
    let c = bn6_content();
    let mut problems = Vec::new();
    for (i, d) in c.defs.forms.iter().enumerate().filter(|(_, d)| d.key.starts_with("bn6:")) {
        let f = FormHandle(i as u16);
        let (kind, form) = (forms::kind(&c, f), &d.record);
        let mut say = |what: &str| problems.push(format!("{}: {what}", d.key));
        if form.base != kind.is_none() {
            say("the base form, and only it, has no kind");
        }
        if form.base == forms::game(&c, f).is_some() {
            say("every form but the base form says whose game's it is");
        }
        if kind.is_some_and(Kind::has_cross) != forms::cross_of(&c, f).is_some() {
            say("a Cross (and one in Beast Out), and only one, names its navi (`cross_of`)");
        }
        if (kind == Some(Kind::Cross)) != forms::in_beast_out(&c, f).is_some() {
            say("a Cross, and only a Cross, names its form in Beast Out (`beast`)");
        }
        if forms::in_beast_out(&c, f).is_some_and(|b| forms::kind(&c, b) != Some(Kind::CrossBeast)) {
            say("its `beast` is a Cross in Beast Out");
        }
        let beast = kind.is_some_and(Kind::is_beast);
        let traits = form.traits;
        for (bit, name, wanted) in [
            (FormTraits::SHOWS_TARGET_MARKER, "shows_target_marker", beast),
            (FormTraits::AFTERIMAGES_STAY, "afterimages_stay", beast),
            (FormTraits::ALT_CHARGE_TIME, "alt_charge_time", beast),
            (FormTraits::DOUBLES_NULL, "doubles_null", kind == Some(Kind::BeastOver)),
            (FormTraits::MOOD_PALETTE, "mood_palette", kind == Some(Kind::Beast)),
            (FormTraits::CONTROLLED, "controlled", kind == Some(Kind::BeastOver)),
        ] {
            if traits.has(bit) != wanted {
                say(&format!("`{name}` is a trait of {}", if wanted { "this kind" } else { "other kinds" }));
            }
        }
    }
    for n in c.defs.navis.iter().filter(|n| n.key.starts_with("bn6:")) {
        let Some(sets) = &n.record.forms else { continue };
        for set in [&sets.gregar, &sets.falzar] {
            let ok = set.crosses.iter().all(|&f| forms::kind(&c, f) == Some(Kind::Cross))
                && set.beast_out.is_none_or(|f| forms::kind(&c, f) == Some(Kind::Beast))
                && set.beast_over.is_none_or(|f| forms::kind(&c, f) == Some(Kind::BeastOver));
            if !ok {
                problems.push(format!("{}: its forms' `crosses` are Crosses, `beast_out` a Beast and `beast_over` a Beast Over", n.key));
            }
        }
    }
    assert!(problems.is_empty(), "{problems:#?}");
}
