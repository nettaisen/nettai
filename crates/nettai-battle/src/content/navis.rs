//! Navis and MegaMan's forms (`define.navi`, `define.form`: docs/design/
//! content-model-v2.md §3.2).

use super::flags::serde_flags;
use super::{BannerId, ChipFamily, CodedChip, Element, SecondaryElements, SpriteId};
use crate::actor::ActorType;
use crate::custom::GameVersion;
use nettai_content_api::{FormHandle, IdentityHandle, NaviHandle, WeaponHandle};
use serde::{Deserialize, Serialize};

/// A navi: MegaMan or a link navi.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NaviData {
    /// The battle sprite (a navi that changes form draws its form's
    /// instead).
    pub sprite: SpriteId,
    pub element: Element,
    #[serde(default)]
    pub weakness: SecondaryElements,
    /// Added to the buster's damage.
    pub buster_bonus: u8,
    /// Ticks of lag after a move, by navi variant (NaviStats' variant).
    pub move_lag: Vec<u8>,
    /// The netbattle result banners.
    pub win_banner: BannerId,
    pub lose_banner: BannerId,
    /// A link navi's face in the emotion window (`sub_801CC34`: a picture
    /// of the link navis' own table, which the window shows in its second
    /// palette in Full Synchro), the mugshot's number; none for MegaMan,
    /// whose face is his form's. Presentation only.
    #[serde(default)]
    pub mugshot: Option<u8>,
    /// Extra height, in whole pixels, of the navi's image as it merges
    /// with MegaMan in a Cross.
    #[serde(default)]
    pub merge_height: i16,
    /// A link navi's own chip, offered on the custom screen once a round.
    #[serde(default)]
    pub own_chip: Option<CodedChip>,
    /// A link navi's damage bonus on its family's chips.
    #[serde(default)]
    pub chip_bonus: Option<NaviChipBonus>,
    /// The no-running message the custom screen shows for the navi (L in
    /// a netbattle).
    #[serde(default)]
    pub run_message: RunMessage,
    /// The chips it charges with A, from a navi level (`sub_800F49E`,
    /// `byte_8021369`).
    #[serde(default)]
    pub charged_chips: Option<NaviChargedChips>,
    /// The chips a charge doubles (`sub_8012AFA`).
    #[serde(default)]
    pub charge_doubles: Option<ChipMatch>,
    /// Its A charge builds up the next Fire chip's damage, up to a limit
    /// by its level (`sub_80F0608`, `byte_802136D`).
    #[serde(default)]
    pub fire_charge: Option<Vec<u8>>,
    #[serde(default)]
    pub traits: NaviTraits,
    /// The forms it changes into (MegaMan's), by game. Where the original
    /// asks whether a navi is MegaMan, the ruleset asks whether it has
    /// forms. (This and what follows are read from the definition by
    /// handle, not with the rest of the record.)
    #[serde(skip)]
    pub forms: Option<NaviForms>,
    /// Its identity: what the object that is this navi is taken for.
    #[serde(skip)]
    pub identity: Option<IdentityHandle>,
    /// The weapons it comes with (`byte_80210DD`): what a Cross change
    /// gives its buttons.
    #[serde(skip)]
    pub weapons: FormWeapons,
    /// The rest of what a Cross change brings it with, fresh; none for a
    /// navi no change can bring.
    #[serde(skip)]
    pub fresh: Option<FreshStats>,
    /// Its HP after a Cross change, by side (`byte_802DD88`).
    #[serde(skip)]
    pub cross_hp: Option<[u16; 2]>,
}

impl NaviData {
    /// It changes form (MegaMan): its forms decide its sprite, element,
    /// weapons and what it wears.
    pub fn changes_form(&self) -> bool {
        self.forms.is_some()
    }
}

/// What the ruleset asks of particular navis. In a content file, a list of
/// names.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct NaviTraits(pub u8);

impl NaviTraits {
    /// A hit's status doesn't take (`sub_801A4D0`'s caller: TomahawkMan).
    pub const STATUS_IMMUNE: u8 = 0x01;
    pub(crate) const NAMES: &[(u32, &str)] = &[(0x01, "status_immune")];

    pub fn has(self, bit: u8) -> bool {
        self.0 & bit != 0
    }
}

serde_flags!(NaviTraits, u8);

/// The forms a navi changes into, by the player's game.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct NaviForms {
    pub gregar: FormSet,
    pub falzar: FormSet,
}

impl NaviForms {
    pub fn of(&self, version: GameVersion) -> &FormSet {
        match version {
            GameVersion::Gregar => &self.gregar,
            GameVersion::Falzar => &self.falzar,
        }
    }
}

/// A game's forms: its Crosses by their number on the custom screen (the
/// save's unlock flags' order), Beast Out and Beast Over.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct FormSet {
    pub crosses: Vec<FormHandle>,
    pub beast_out: Option<FormHandle>,
    pub beast_over: Option<FormHandle>,
}

/// A navi's stats when a Cross change brings it fresh (`byte_80210DD`,
/// with its weapons): its HP, its body's programs, the first barrier, its
/// Mega and Giga levels, and the damage of its B+Back special.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct FreshStats {
    pub hp: u16,
    pub super_armor: bool,
    pub float_shoes: bool,
    pub air_shoes: bool,
    pub undershirt: bool,
    pub first_barrier: u8,
    pub mega_level: u8,
    pub giga_level: u8,
    pub back_special_damage: u16,
}

/// A link navi's damage bonus on the damaging chips of its family
/// (`sub_800F09E`), by the navi's level (`byte_8021300`, 15 a navi).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NaviChipBonus {
    pub family: ChipFamily,
    /// Dimming chips of the family count too.
    #[serde(default)]
    pub dimming_chips: bool,
    pub by_level: Vec<u8>,
}

/// The chips a link navi charges with A: its family's damaging chips, from
/// a navi level on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NaviChargedChips {
    pub family: ChipFamily,
    pub from_level: u8,
}

/// The chips a rule is about: an element's, or a family's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChipMatch {
    Element(Element),
    Family(ChipFamily),
}

/// What kind of form one of MegaMan's is.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FormKind {
    /// His own.
    #[default]
    Base,
    /// A Cross: merged with a link navi.
    Cross,
    /// Beast Out.
    Beast,
    /// A Cross in Beast Out.
    CrossBeast,
    /// Beast Over: the navi acts on its own.
    BeastOver,
}

impl FormKind {
    /// Beast Out, with or without a Cross, or Beast Over.
    pub fn is_beast(self) -> bool {
        matches!(self, FormKind::Beast | FormKind::CrossBeast | FormKind::BeastOver)
    }

    /// Beast Out, with or without a Cross.
    pub fn is_beast_out(self) -> bool {
        matches!(self, FormKind::Beast | FormKind::CrossBeast)
    }

    /// Beast Over.
    pub fn is_beast_over(self) -> bool {
        self == FormKind::BeastOver
    }

    /// A Cross is on (a Cross, or one in Beast Out).
    pub fn has_cross(self) -> bool {
        matches!(self, FormKind::Cross | FormKind::CrossBeast)
    }
}

/// One of MegaMan's forms.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormData {
    pub sprite: SpriteId,
    pub element: Element,
    #[serde(default)]
    pub weakness: SecondaryElements,
    /// Added to the buster's damage.
    pub buster_bonus: u8,
    pub kind: FormKind,
    /// Whose game's form it is: the Beast's roar and Beast Over's glow.
    #[serde(default)]
    pub game: Option<GameVersion>,
    /// MegaMan's palette in it (`byte_80203EA`: a Cross's; the base form's
    /// and Beast Out's follow the mood).
    #[serde(default)]
    pub palette: u8,
    /// Its faces in the emotion window (`sub_801E6A8`), by emotion.
    /// Presentation only.
    #[serde(default, deserialize_with = "faces")]
    pub mugshot: Option<Faces>,
    /// The lines of a Cross's description, which R shows in the Cross
    /// window (its text is the content's strings): the box takes keys a
    /// tick later for each, as for a chip's; none counts as three. The
    /// define phase counts it from the content's own strings.
    #[serde(skip, default = "super::strings::three_lines")]
    pub description_lines: u8,
    /// The damage it adds to a family's damaging chips (`sub_800EF34`).
    #[serde(default)]
    pub chip_bonus: Option<FormChipBonus>,
    /// The damage it adds to damaging Null chips when the family's bonus
    /// doesn't apply, outside battle mode 1 (Beast Out's).
    #[serde(default)]
    pub null_bonus: u16,
    /// The chips it charges with A (`sub_8013236`).
    #[serde(default)]
    pub charged_chips: Vec<ChargedChips>,
    /// What a charged chip gains in it (`sub_8012C7C`).
    #[serde(default)]
    pub charged_bonus: ChargedBonus,
    /// The chips a charge doubles (`sub_8012AFA`).
    #[serde(default)]
    pub charge_doubles: Option<ChipMatch>,
    /// The chips whose use heals a twentieth of the base HP
    /// (`sub_800E2FC`'s caller: not dimming chips).
    #[serde(default)]
    pub chip_heals: Option<ChipMatch>,
    /// Its A charge builds up the next Fire chip's damage, up to this
    /// much (`sub_80F0608`).
    #[serde(default)]
    pub fire_charge: Option<u16>,
    /// What the status reset gives it (`sub_8014536`), and what a NaviCust
    /// change gives back (`sub_801469C`; none: the reset's, without the
    /// lock-on marker).
    #[serde(default)]
    pub status_reset: FormEffects,
    #[serde(default)]
    pub navicust_refresh: Option<FormEffects>,
    /// The height it floats at, in whole pixels (`sub_80F0608`).
    #[serde(default)]
    pub hover: i16,
    /// The shots of the buster volley the Cross special's controller
    /// fires in it (`sub_802D4F0`).
    #[serde(default)]
    pub special_volley: u16,
    /// A Cross change that finds the navi in this animation lets go of it
    /// and of what it holds (`sub_8014B18`: GroundCross's drill).
    #[serde(default)]
    pub cross_release_anim: Option<u8>,
    #[serde(default)]
    pub traits: FormTraits,
    /// The navi a Cross is made with: its image merges with MegaMan.
    /// (This and what follows are read from the definition by handle, not
    /// with the rest of the record.)
    #[serde(skip)]
    pub cross_of: Option<NaviHandle>,
    /// A Cross's form in Beast Out.
    #[serde(skip)]
    pub beast: Option<FormHandle>,
    /// What a weakness hit drops it to (`sub_8015766`); none: it stays.
    #[serde(skip)]
    pub breaks_to: Option<FormHandle>,
    #[serde(skip)]
    pub weapons: FormWeapons,
    /// Its identity (the base form has none of its own: the navi's).
    #[serde(skip)]
    pub identity: Option<IdentityHandle>,
}

/// A form's faces in the emotion window, by its navi's emotion
/// (`sub_8015B54`): the mugshots' numbers. An emotion without a face of
/// its own shows the normal one. (In BN6 the base form has MegaMan's five,
/// a Cross a tired one besides, a Beast a Full Synchro one: `sub_801E6A8`
/// adds 5 or 1 to `byte_801E700`'s picture.)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Faces {
    pub normal: u8,
    #[serde(default)]
    pub angry: Option<u8>,
    #[serde(default)]
    pub tired: Option<u8>,
    #[serde(default)]
    pub full_synchro: Option<u8>,
    #[serde(default)]
    pub worn_out: Option<u8>,
}

impl Faces {
    /// The face for `emotion`.
    pub fn of(&self, emotion: crate::kinds::player::Emotion) -> u8 {
        use crate::kinds::player::Emotion;
        let face = match emotion {
            Emotion::Normal => None,
            Emotion::Angry => self.angry,
            Emotion::Tired => self.tired,
            Emotion::FullSynchro => self.full_synchro,
            Emotion::WornOut => self.worn_out,
        };
        face.unwrap_or(self.normal)
    }
}

/// A form definition's `mugshot`: one face, or faces by emotion.
fn faces<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Faces>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Spec {
        One(u8),
        ByEmotion(Faces),
    }
    Ok(Option::<Spec>::deserialize(d)?.map(|s| match s {
        Spec::One(normal) => Faces { normal, ..Faces::default() },
        Spec::ByEmotion(f) => f,
    }))
}

/// A navi's no-running message (L on the custom screen in a netbattle).
/// Its text is the content's strings (`Content::strings`); the define
/// phase counts them here.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RunMessage {
    /// The characters in each of its lines (up to three), which set how
    /// long it prints (docs/engine/custom-screen.md §3.5).
    #[serde(skip)]
    pub counts: Vec<u8>,
    /// Which characters move the speaker's mouth, by line: bit k for the
    /// line's character k (`strings::talking`). Presentation.
    #[serde(skip)]
    pub talking: [u32; 3],
    /// Who says it: the portrait beside the box (a sprite whose
    /// animations are its faces: still, idle, talking). Presentation.
    #[serde(default)]
    pub portrait: Option<SpriteId>,
}

impl FormData {
    /// What a NaviCust change gives the form back.
    pub fn refresh_effects(&self) -> FormEffects {
        self.navicust_refresh.unwrap_or(FormEffects(self.status_reset.0 & !FormEffects::LOCKON_MARKER))
    }
}

/// The damage a form adds to a family's damaging chips.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormChipBonus {
    pub family: ChipFamily,
    pub damage: u16,
    /// Dimming chips of the family count too.
    #[serde(default)]
    pub dimming_chips: bool,
}

/// Chips a form charges with A: a family's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChargedChips {
    pub family: ChipFamily,
    /// Only its damaging chips that aren't dimming chips (false: any).
    #[serde(default = "yes")]
    pub damaging: bool,
    /// And the chips with the `element_sword` trait.
    #[serde(default)]
    pub element_swords: bool,
}

fn yes() -> bool {
    true
}

/// What a charged chip gains in a form.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChargedBonus {
    #[serde(default)]
    pub damage: u16,
    /// It paralyzes.
    #[serde(default)]
    pub paralyzes: bool,
}

/// What a form's status reset gives the navi. In a content file, a list of
/// names.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct FormEffects(pub u16);

impl FormEffects {
    /// The statuses end (not in battle mode 1).
    pub const CLEAR_STATUSES: u16 = 0x001;
    pub const SUPER_ARMOR: u16 = 0x002;
    pub const AIR_SHOES: u16 = 0x004;
    pub const FLOAT_SHOES: u16 = 0x008;
    /// The body floats (its collision type).
    pub const FLOATING_BODY: u16 = 0x010;
    /// Untouchable (ObjectFlags1 0x08000000): no hit reaches it, and poison
    /// panels don't hurt it.
    pub const UNTOUCHABLE: u16 = 0x020;
    /// The Beast's lock-on marker.
    pub const LOCKON_MARKER: u16 = 0x040;
    /// Invulnerable for good.
    pub const INVULNERABLE: u16 = 0x080;
    /// The berserk controller starts over.
    pub const BERSERK: u16 = 0x100;
    pub(crate) const NAMES: &[(u32, &str)] = &[
        (0x001, "clear_statuses"),
        (0x002, "super_armor"),
        (0x004, "air_shoes"),
        (0x008, "float_shoes"),
        (0x010, "floating_body"),
        (0x020, "untouchable"),
        (0x040, "lockon_marker"),
        (0x080, "invulnerable"),
        (0x100, "berserk"),
    ];

    pub fn has(self, bit: u16) -> bool {
        self.0 & bit != 0
    }
}

serde_flags!(FormEffects, u16);

/// What the ruleset asks of particular forms. In a content file, a list of
/// names.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct FormTraits(pub u8);

impl FormTraits {
    /// A hit's status doesn't take (TomahawkCross).
    pub const STATUS_IMMUNE: u8 = 0x01;
    /// Its damaging Null chips delete at once what has a 4 in its HP
    /// (`sub_8012C4A`: EraseCross).
    pub const ERASES: u8 = 0x02;
    /// Its charged sword runs inside the Beast Out rush (`sub_800FB54`:
    /// SlashCross in Beast Out).
    pub const CHARGED_SWORD_RUSH: u8 = 0x04;
    /// The custom screen deals one more chip for each screen spent in it,
    /// up to three (`sub_802A49C`: ChargeCross).
    pub const EXTRA_CHIPS: u8 = 0x08;
    /// The custom screen has the scrap button (`sub_8027F10`: DustCross).
    pub const SCRAP_BUTTON: u8 = 0x10;
    /// Its held buster doesn't fire while its B+Back special is asked for
    /// (TenguCross and DustCross in Beast Out).
    pub const SPECIAL_HOLDS_BUSTER: u8 = 0x20;
    pub(crate) const NAMES: &[(u32, &str)] = &[
        (0x01, "status_immune"),
        (0x02, "erases"),
        (0x04, "charged_sword_rush"),
        (0x08, "extra_chips"),
        (0x10, "scrap_button"),
        (0x20, "special_holds_buster"),
    ];

    pub fn has(self, bit: u8) -> bool {
        self.0 & bit != 0
    }
}

serde_flags!(FormTraits, u8);

/// A form's or navi's weapons, by the button that uses each (none: the
/// original's 0xFF).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct FormWeapons {
    /// The A button in battle mode 9.
    pub mode9_a: Option<WeaponHandle>,
    /// The A-button charge (charged chips).
    pub a_charge: Option<WeaponHandle>,
    /// The B-button buster.
    pub buster: Option<WeaponHandle>,
    /// The charged shot.
    pub charge_shot: Option<WeaponHandle>,
    /// The B+Back special.
    pub back_special: Option<WeaponHandle>,
    /// The A-button charge for Null-family chips in Beast forms.
    pub alt_a_charge: Option<WeaponHandle>,
}

impl FormWeapons {
    /// The slots' names in a definition's `weapons`.
    pub const SLOTS: [&'static str; 6] = ["mode9_a", "a_charge", "buster", "charge_shot", "back_special", "alt_a_charge"];

    /// The slot named `slot`.
    pub fn slot_mut(&mut self, slot: &str) -> Option<&mut Option<WeaponHandle>> {
        Some(match slot {
            "mode9_a" => &mut self.mode9_a,
            "a_charge" => &mut self.a_charge,
            "buster" => &mut self.buster,
            "charge_shot" => &mut self.charge_shot,
            "back_special" => &mut self.back_special,
            "alt_a_charge" => &mut self.alt_a_charge,
            _ => return None,
        })
    }
}

/// An actor record (`sub_80182B4`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NaviRecord {
    pub version: u8,
    pub actor_type: ActorType,
    /// Selects per-navi hooks and tables.
    pub ai_index: u8,
}

/// A sprite attach point, in pixels, x toward the facing side. In a
/// content file, `[x, y]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AttachPoint {
    pub x: i8,
    pub y: i8,
}

impl Serialize for AttachPoint {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        [self.x, self.y].serialize(s)
    }
}

impl<'de> Deserialize<'de> for AttachPoint {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<AttachPoint, D::Error> {
        let [x, y] = <[i8; 2]>::deserialize(d)?;
        Ok(AttachPoint { x, y })
    }
}

/// A navi definition's record: what it gives by value. (What it names by
/// handle is read with the registries: `Defs::build`.)
pub(crate) fn read_navi(
    d: &nettai_content_api::Definition,
    r: &super::reader::SpecReader,
) -> Result<NaviData, nettai_content_api::ContentError> {
    use serde_json::Value as Json;
    let err = |m: String| super::reader::err(d, m);
    // (`actions` are the content's own.)
    let mut o = super::reader::fields(
        d,
        r,
        &["id", "identity", "banners", "own_chip", "actions", "weapons", "fresh", "cross_hp", "forms"],
    )?;
    let banners = d.spec.field("banners");
    for (field, which) in [("win_banner", "win"), ("lose_banner", "lose")] {
        let b = r.json(banners.field(which), &format!("navi {}.banners.{which}", d.key)).map_err(err)?;
        o.insert(field.into(), b);
    }
    let own = d.spec.field("own_chip");
    if !own.is_nil() {
        o.insert("own_chip".into(), r.json(own, &format!("navi {}.own_chip", d.key)).map_err(err)?);
    }
    serde_json::from_value(Json::Object(o)).map_err(|m| err(m.to_string()))
}

/// A form definition's record, likewise.
pub(crate) fn read_form(
    d: &nettai_content_api::Definition,
    r: &super::reader::SpecReader,
) -> Result<FormData, nettai_content_api::ContentError> {
    use serde_json::Value as Json;
    // (`buster_arm` is the content's own: the arm a navi raises.)
    let o = super::reader::fields(d, r, &["id", "identity", "cross_of", "beast", "breaks_to", "weapons", "buster_arm"])?;
    let form: FormData = serde_json::from_value(Json::Object(o)).map_err(|m| super::reader::err(d, m))?;
    if form.kind != FormKind::Base && form.game.is_none() {
        return Err(super::reader::err(d, "a form that is not the base form says whose `game` it is (gregar, falzar)"));
    }
    Ok(form)
}

/// A navi definition's `fresh`: what a Cross change brings it with.
pub(crate) fn read_fresh(d: &nettai_content_api::Definition) -> Result<Option<FreshStats>, nettai_content_api::ContentError> {
    use nettai_content_api::{ContentError, Data};
    let what = |m: String| ContentError::new(format!("{}.luau: navi {}: {m}", d.module, d.key));
    let fresh = match d.spec.field("fresh") {
        Data::Nil => return Ok(None),
        v @ Data::Map(_) => v,
        other => return Err(what(format!("`fresh` is {other:?}, not a table"))),
    };
    let number = |field: &str, max: i64| -> Result<i64, ContentError> {
        match fresh.field(field) {
            Data::Int(i) if (0..=max).contains(i) => Ok(*i),
            other => Err(what(format!("fresh.{field} is {other:?}, not a number up to {max}"))),
        }
    };
    let flag = |field: &str| -> Result<bool, ContentError> {
        match fresh.field(field) {
            Data::Nil => Ok(false),
            Data::Bool(b) => Ok(*b),
            other => Err(what(format!("fresh.{field} is {other:?}, not true or false"))),
        }
    };
    Ok(Some(FreshStats {
        hp: number("hp", 0xFFFF)? as u16,
        super_armor: flag("super_armor")?,
        float_shoes: flag("float_shoes")?,
        air_shoes: flag("air_shoes")?,
        undershirt: flag("undershirt")?,
        first_barrier: number("first_barrier", 0xFF)? as u8,
        mega_level: number("mega_level", 0xFF)? as u8,
        giga_level: number("giga_level", 0xFF)? as u8,
        back_special_damage: match fresh.field("back_special_damage") {
            Data::Nil => 0,
            _ => number("back_special_damage", 0xFFFF)? as u16,
        },
    }))
}

/// A navi definition's `cross_hp`: its HP after a Cross change, by side.
pub(crate) fn read_cross_hp(d: &nettai_content_api::Definition) -> Result<Option<[u16; 2]>, nettai_content_api::ContentError> {
    use nettai_content_api::{ContentError, Data};
    let what = || ContentError::new(format!("{}.luau: navi {}: `cross_hp` is two HP values, by side", d.module, d.key));
    match d.spec.field("cross_hp") {
        Data::Nil => Ok(None),
        Data::List(items) => match items.as_slice() {
            [Data::Int(a), Data::Int(b)] if (0..=0xFFFF).contains(a) && (0..=0xFFFF).contains(b) => Ok(Some([*a as u16, *b as u16])),
            _ => Err(what()),
        },
        _ => Err(what()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kinds::player::Emotion;

    #[test]
    fn a_form_shows_its_face_for_an_emotion_or_its_normal_one() {
        #[derive(Deserialize)]
        struct Form {
            #[serde(default, deserialize_with = "faces")]
            mugshot: Option<Faces>,
        }
        let read = |json: &str| serde_json::from_str::<Form>(json).unwrap().mugshot;
        // One face, whatever the emotion.
        let one = read(r#"{ "mugshot": 15 }"#).unwrap();
        assert_eq!([Emotion::Normal, Emotion::Tired, Emotion::FullSynchro].map(|e| one.of(e)), [15; 3]);
        // A Cross's: its own, and a tired one.
        let cross = read(r#"{ "mugshot": { "normal": 5, "tired": 10 } }"#).unwrap();
        assert_eq!([Emotion::Normal, Emotion::Angry, Emotion::Tired].map(|e| cross.of(e)), [5, 5, 10]);
        assert_eq!(read("{}"), None);
    }
}
