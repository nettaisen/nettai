//! EXE6's forms say of each other what EXE6's rules mean
//! (docs/design/rules-in-luau.md, S7b): rules/forms's kinds and games,
//! a Cross's navi and form in Beast Out, the navis' form sets, and the
//! framework's behavior traits that follow from the kinds (the original's
//! tests of the form's number). The engine checks only the extensions'
//! types; these are EXE6's own rules, checked of content/exe6 as compat
//! reads it (`exe6_compat::forms`).

use exe6_compat::GameVersion;
use exe6_compat::forms::{self, Kind};
use nettai_battle::content::FormTraits;
use nettai_content_api::FormHandle;
use nettai_match::ids::in_game;
use nettai_match::testing::exe6_content;

#[test]
fn exe6_forms_agree_with_their_kinds() {
    let c = exe6_content();
    let mut problems = Vec::new();
    for (i, d) in c.defs.forms.iter().enumerate().filter(|(_, d)| in_game(&c, "exe6", &d.key)) {
        let f = FormHandle(i as u16);
        let (kind, form) = (forms::kind(&c, f), &d.record);
        let mut say = |what: &str| problems.push(format!("{}: {what}", d.key));
        if form.base != kind.is_none() {
            say("the base form, and only it, has no kind");
        }
        if form.base == forms::game(&c, f).is_some() {
            say("every form but the base form says whose game's it is (its `version`)");
        }
        // (A link navi is of its Cross's game: its `version`.)
        if let (Some(Kind::Cross), Some(navi)) = (kind, forms::cross_of(&c, f))
            && c.navi(navi).version != form.version
        {
            say("its navi (`cross_of`) is of its version");
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
    for (i, n) in c.defs.navis.iter().enumerate().filter(|(_, n)| in_game(&c, "exe6", &n.key)) {
        let navi = nettai_content_api::NaviHandle(i as u16);
        let sets = [GameVersion::Gregar, GameVersion::Falzar].map(|g| forms::set(&c, navi, g));
        if n.record.forms.is_some() != sets.iter().any(Option::is_some) {
            problems.push(format!("{}: a navi with forms has EXE6's sets (`forms.gregar`, `forms.falzar`), and only one", n.key));
        }
        for (game, set) in [GameVersion::Gregar, GameVersion::Falzar].into_iter().zip(&sets) {
            let Some(set) = set else { continue };
            let ok = set.crosses.iter().all(|&f| forms::kind(&c, f) == Some(Kind::Cross))
                && set.beast_out.is_none_or(|f| forms::kind(&c, f) == Some(Kind::Beast))
                && set.beast_over.is_none_or(|f| forms::kind(&c, f) == Some(Kind::BeastOver));
            if !ok {
                problems.push(format!("{}: its forms' `crosses` are Crosses, `beast_out` a Beast and `beast_over` a Beast Over", n.key));
            }
            // A game's set is of that game's forms (a Cross's number in
            // the Cross window is its place in the set).
            if set.crosses.iter().chain(&set.beast_out).chain(&set.beast_over).any(|&f| forms::game(&c, f) != Some(game)) {
                problems.push(format!("{}: its {game:?} forms are that game's (their `version`)", n.key));
            }
        }
        // (Only a Cross's navi is of one version.)
        let crossed = (0..c.defs.forms.len() as u16).map(FormHandle).any(|f| forms::cross_of(&c, f) == Some(navi));
        if n.record.version.is_some() != crossed {
            problems.push(format!("{}: a link navi with a Cross, and only one, says its `version`", n.key));
        }
    }
    assert!(problems.is_empty(), "{problems:#?}");
}

/// The Crosses' strings: what the Cross window shows (their names and
/// descriptions) the own language's table has for every EXE6 Cross, and the
/// table names no other EXE6 form (nothing shows another form's).
#[test]
fn exe6_crosses_have_their_strings() {
    let c = exe6_content();
    let crosses: Vec<FormHandle> = (0..c.defs.forms.len() as u16)
        .map(FormHandle)
        .filter(|&f| in_game(&c, "exe6", &c.defs.form(f).key) && forms::kind(&c, f) == Some(Kind::Cross))
        .collect();
    assert_eq!(crosses.len(), 10);
    for &f in &crosses {
        let key = &c.defs.form(f).key;
        let s = c.strings.form(key).unwrap_or_else(|| panic!("{key} has no strings"));
        assert!(s.name.is_some() && s.description.is_some(), "{key}'s name and description");
    }
    for key in c.strings.forms.keys() {
        let f = c.defs.form_by_key(key).unwrap_or_else(|| panic!("{key}: no form has this key"));
        assert!(crosses.contains(&f), "{key}: not a Cross (nothing shows another form's strings)");
    }
}
