//! What BN6's systems say of its forms (docs/design/rules-in-luau.md, S7b
//! and S7c):
//! their extensions of BN6's form definitions, which the engine checks and
//! never reads. The forms system's: a form's kind, whose game's form it is,
//! the navi a Cross is made with, the animation a change into a Cross lets
//! the navi go of. The beast system's: a Cross's form in Beast Out, the
//! Cross special's buster volley. The cross system's: ChargeCross's extra
//! chips, DustCross's scrap button. And a navi's sets by game (its `forms`
//! table's `gregar` and `falzar`: its Crosses, Beast Out and Beast Over),
//! which the engine leaves to BN6 too. Tools read them here, by the
//! fields' names in content/bn6.

use nettai_battle::content::Content;
use nettai_battle::custom::GameVersion;
use nettai_content_api::{Data, FormHandle, NaviHandle, Registry};

/// A BN6 form's kind (the forms system's `kind`). The base form, and any
/// other game's form, has none.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    /// A Cross: merged with a link navi (forms 1 to 10).
    Cross,
    /// Beast Out (0x0B, 0x0C).
    Beast,
    /// A Cross in Beast Out (0x0D to 0x16).
    CrossBeast,
    /// Beast Over: the navi acts on its own (0x17, 0x18).
    BeastOver,
}

impl Kind {
    /// Beast Out, with or without a Cross, or Beast Over.
    pub fn is_beast(self) -> bool {
        matches!(self, Kind::Beast | Kind::CrossBeast | Kind::BeastOver)
    }

    /// A Cross is on (a Cross, or one in Beast Out).
    pub fn has_cross(self) -> bool {
        matches!(self, Kind::Cross | Kind::CrossBeast)
    }
}

/// Form `form`'s extension field `field`, if it says one.
fn field<'a>(content: &'a Content, form: FormHandle, field: &str) -> Option<&'a Data> {
    content.defs.extension(Registry::Form, &content.defs.form(form).key, field)
}

/// Form `form`'s kind (none: the base form, or another game's form).
pub fn kind(content: &Content, form: FormHandle) -> Option<Kind> {
    match field(content, form, "kind")? {
        Data::Str(k) => Some(match k.as_str() {
            "cross" => Kind::Cross,
            "beast" => Kind::Beast,
            "cross_beast" => Kind::CrossBeast,
            "beast_over" => Kind::BeastOver,
            other => panic!("form {}'s kind {other:?} is none of BN6's (the forms system checks it)", content.defs.form(form).key),
        }),
        other => panic!("form {}'s kind is {other:?} (the forms system checks it)", content.defs.form(form).key),
    }
}

/// The form is a Beast form (Beast Out, with or without a Cross, or Beast
/// Over).
pub fn is_beast(content: &Content, form: FormHandle) -> bool {
    kind(content, form).is_some_and(Kind::is_beast)
}

/// Whose game's form it is (the Beast's roar; none: the base form, or
/// another game's form).
pub fn game(content: &Content, form: FormHandle) -> Option<GameVersion> {
    match field(content, form, "game")? {
        Data::Str(g) if g == "gregar" => Some(GameVersion::Gregar),
        Data::Str(g) if g == "falzar" => Some(GameVersion::Falzar),
        other => panic!("form {}'s game is {other:?} (the forms system checks it)", content.defs.form(form).key),
    }
}

/// The navi a Cross (and one in Beast Out) is made with.
pub fn cross_of(content: &Content, form: FormHandle) -> Option<NaviHandle> {
    match field(content, form, "cross_of")? {
        Data::Ref(Registry::Navi, key) => content.defs.navi_by_key(key),
        other => panic!("form {}'s cross_of is {other:?}", content.defs.form(form).key),
    }
}

/// A Cross's form in Beast Out.
pub fn in_beast_out(content: &Content, form: FormHandle) -> Option<FormHandle> {
    match field(content, form, "beast")? {
        Data::Ref(Registry::Form, key) => content.defs.form_by_key(key),
        other => panic!("form {}'s beast is {other:?}", content.defs.form(form).key),
    }
}

/// The shots of the buster volley the Cross special's controller fires in
/// the form (`sub_802D4F0`; none said: 0).
pub fn special_volley(content: &Content, form: FormHandle) -> u16 {
    field(content, form, "special_volley").and_then(Data::int).map_or(0, |v| v as u16)
}

/// The animation a change into a Cross lets the navi go of
/// (`sub_8014B18`: GroundCross's drill).
pub fn cross_release_anim(content: &Content, form: FormHandle) -> Option<u8> {
    field(content, form, "cross_release_anim").and_then(Data::int).map(|v| v as u8)
}

/// A boolean extension field (`extra_chips`, `scrap_button`,
/// `charged_sword_rush`): whether the form says it.
pub fn says(content: &Content, form: FormHandle, name: &str) -> bool {
    matches!(field(content, form, name), Some(Data::Bool(true)))
}

/// A navi's forms in one of BN6's games (its `forms.gregar` or
/// `forms.falzar`): its Crosses by their number on the custom screen (the
/// save's unlock flags' order), Beast Out and Beast Over.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Set {
    pub crosses: Vec<FormHandle>,
    pub beast_out: Option<FormHandle>,
    pub beast_over: Option<FormHandle>,
}

/// Navi `navi`'s set in `game` (none: it has no forms there).
pub fn set(content: &Content, navi: NaviHandle, game: GameVersion) -> Option<Set> {
    let key = &content.defs.navi(navi).key;
    let d = content.defs.definitions.get(Registry::Navi, key)?;
    let name = match game {
        GameVersion::Gregar => "gregar",
        GameVersion::Falzar => "falzar",
    };
    let g = d.spec.field("forms").field(name);
    if matches!(g, Data::Nil) {
        return None;
    }
    let form = |v: &Data| match v {
        Data::Ref(Registry::Form, k) => content.defs.form_by_key(k),
        _ => None,
    };
    let crosses = match g.field("crosses") {
        Data::List(items) => items.iter().filter_map(form).collect(),
        _ => Vec::new(),
    };
    Some(Set { crosses, beast_out: form(g.field("beast_out")), beast_over: form(g.field("beast_over")) })
}

/// Navi `navi`'s Cross with number `cross` in `game` (none: no such Cross).
pub fn cross(content: &Content, navi: NaviHandle, game: GameVersion, cross: u8) -> Option<FormHandle> {
    set(content, navi, game)?.crosses.get(cross as usize).copied()
}

/// Navi `navi`'s Beast Out form in `game`.
pub fn beast_out(content: &Content, navi: NaviHandle, game: GameVersion) -> Option<FormHandle> {
    set(content, navi, game)?.beast_out
}

/// Navi `navi`'s Beast Over form in `game`.
pub fn beast_over(content: &Content, navi: NaviHandle, game: GameVersion) -> Option<FormHandle> {
    set(content, navi, game)?.beast_over
}
