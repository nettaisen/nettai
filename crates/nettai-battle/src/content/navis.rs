//! Navis and MegaMan's forms (the root's `navis` and `forms`: docs/design/
//! content-model-v2.md §3.2).

use super::flags::serde_flags;
use super::{BannerId, ChipFamily, CodedChip, Element, SecondaryElements, SpriteId};
use crate::actor::ActorType;
use nettai_content_api::{FormHandle, IdentityHandle, WeaponHandle};
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
    pub mugshot: Option<super::MugshotId>,
    /// The version of its game a link navi belongs to (EXE6's Gregar's or
    /// Falzar's link navis), by the name its game's pack keeps a version's
    /// pictures under; none: every version's. Presentation only (a console
    /// of another version has no portrait of it).
    #[serde(default)]
    pub version: Option<String>,
    /// Extra height, in whole pixels, of the navi's image as it merges
    /// with MegaMan in a Cross.
    #[serde(default)]
    pub merge_height: i16,
    /// A link navi's own chip, offered on the custom screen once a round.
    #[serde(default)]
    pub own_chip: Option<CodedChip>,
    /// The custom screen's offer of the own chip draws once from the
    /// console's RNG (EXE5's, which picks between its table's two entries
    /// for the navi, the same chip, with the draw).
    #[serde(default)]
    pub own_chip_draws: bool,
    /// A link navi's damage bonus on its family's chips (its amount a
    /// function of the side: `NaviDef::given`).
    #[serde(default)]
    pub chip_bonus: Option<NaviChipBonus>,
    /// MegaMan's bonus on a family's damaging chips used standing on a
    /// type of panel, in a form with no `chip_bonus` of its own (EXE5's: Aqua
    /// chips on sea, 0x0800D0A6); the use turns the panel Normal.
    #[serde(default)]
    pub panel_bonus: Option<PanelChipBonus>,
    /// The no-running message the custom screen shows for the navi (L in
    /// a netbattle).
    #[serde(default)]
    pub run_message: RunMessage,
    /// The chips it charges with A (EXE6's link navis' from a navi level,
    /// `sub_800F49E` and `byte_8021369`: `when`, a function of the side,
    /// `NaviDef::given`; EXE5's team navis', 0x0801090A's tests by navi).
    #[serde(default)]
    pub charged_chips: Option<NaviChargedChips>,
    /// What a chip it charged gains (EXE5's team navis', 0x080103D0's tests
    /// by navi), in a form that states none: a form's own `charged_bonus`
    /// comes first, as the original tests the soul before the navi.
    #[serde(default)]
    pub charged_bonus: Option<ChargedBonus>,
    /// The chips a charge doubles (`sub_8012AFA`).
    #[serde(default)]
    pub charge_doubles: Option<ChipMatch>,
    #[serde(default)]
    pub traits: NaviTraits,
    /// What lifts its charge glow: while its animation is one of these the
    /// glow shows that much above its attach point 0 (EXE5's GyroMan in the
    /// air: the charge glow's test of his name and animation, 0x080E0E58).
    #[serde(default)]
    pub charge_glow_lift: Option<ChargeGlowLift>,
    /// What its sprite's palettes go by, for a navi that doesn't change
    /// form: its Full Synchro palette is 4 of them and its can't-charge
    /// palette 1 (EXE5's 0x0801D737 by navi; EXE6's `byte_80212BB` is all
    /// ones, as none given). Presentation only.
    #[serde(default = "one_palette")]
    pub palette_step: u8,
    /// Its forms ([`NaviForms`]): where the original asks whether a navi is
    /// MegaMan, the rules ask whether it has forms. (This and what
    /// follows are read from the definition by handle, not with the rest of
    /// the record.)
    #[serde(skip)]
    pub forms: Option<NaviForms>,
    /// Its identity: what the object that is this navi is taken for.
    #[serde(skip)]
    pub identity: Option<IdentityHandle>,
    /// The weapons it comes with (`byte_80210DD`): what a navi switch
    /// gives its buttons.
    #[serde(skip)]
    pub weapons: FormWeapons,
    /// The rest of what a navi switch brings it with, fresh; none for a
    /// navi no change can bring.
    #[serde(skip)]
    pub fresh: Option<FreshStats>,
    /// Its HP after a navi switch, by side (`byte_802DD88`).
    #[serde(skip)]
    pub switch_hp: Option<[u16; 2]>,
    /// What the save's reload gives it by its link navi level
    /// (docs/engine/link-navis.md), which its game's rules apply as a
    /// round is set up. Tools read its levels' range (`last_level`); no
    /// battle reads it.
    #[serde(skip)]
    pub levels: Option<NaviLevels>,
    /// What the story gives it by its level (EXE5's team navis), which its
    /// game's rules apply as a round is set up. Tools read its levels'
    /// range (`last_level`) and its HP at a level (nettai-match's `story`);
    /// no battle reads it.
    #[serde(skip)]
    pub story: Option<NaviStory>,
}

fn one_palette() -> u8 {
    1
}

/// What the story gives a navi (EXE5's team navis): its HP by the story's
/// progress (the routine a map script calls as the story moves on, and a
/// new game's start; its table's row), and its level, which its damage
/// rows read: that progress, up to `max_level`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct NaviStory {
    /// The HP (current, maximum and base), from progress 0.
    pub hp: Vec<u16>,
    pub max_level: u8,
}

impl NaviStory {
    /// The HP at `level`: below the last level, the progress is the level;
    /// at the last, the story is taken as done (the table's last entry).
    /// None past the levels.
    pub fn hp_at(&self, level: u8) -> Option<u16> {
        match level.cmp(&self.max_level) {
            std::cmp::Ordering::Less => self.hp.get(level as usize).copied(),
            std::cmp::Ordering::Equal => self.hp.last().copied(),
            std::cmp::Ordering::Greater => None,
        }
    }
}

/// A navi's levels: what the save's reload (`reloadCurNaviBaseStats_8120df0`)
/// gives it by its link navi level.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct NaviLevels {
    /// The base and maximum HP the reload sets before the level's, by the
    /// story's progress (`off_8120F44`; `sub_8121108`'s index, 0 to 6).
    pub base_hp: Vec<u16>,
    /// What each level adds, from level 0 (`pt_8121200`'s scripts, summed
    /// by `sub_8121154`).
    pub by_level: Vec<LevelGain>,
}

/// What a level adds (`sub_8123208`): HP to the maximum, the buster's
/// levels (to 4 at most), the custom level (to 8) and the Mega level (to
/// 10); the abilities it gives; the B+Back special it sets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct LevelGain {
    pub hp: u16,
    pub attack: u8,
    pub rapid: u8,
    pub charge: u8,
    pub custom_level: u8,
    pub mega_level: u8,
    pub super_armor: bool,
    pub float_shoes: bool,
    pub air_shoes: bool,
    pub back_special: Option<WeaponHandle>,
}

impl NaviData {
    /// It changes form (MegaMan): its forms decide its sprite, element,
    /// weapons and what it wears.
    pub fn changes_form(&self) -> bool {
        self.forms.is_some()
    }

    /// The last of its levels, by what its definition says a level gives
    /// it (`levels`: a navi code's, EXE6's link navis' and MegaMan's;
    /// `story`: the story's progress, EXE5's team navis'); none: a level
    /// gives it nothing. A tool's view of the navi (a side's level's range).
    pub fn last_level(&self) -> Option<u8> {
        match (&self.levels, &self.story) {
            (Some(levels), _) => Some(levels.by_level.len().saturating_sub(1) as u8),
            (None, Some(story)) => Some(story.max_level),
            (None, None) => None,
        }
    }
}

/// What the rules ask of particular navis. In a content file, a list of
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

/// A navi's forms (`forms`): that it has any says the navi changes form
/// (MegaMan: where the original asks whether a navi is MegaMan). The table
/// is its game's, sets by name (EXE6's by game, `gregar` and `falzar`: its
/// Crosses, Beast Out and Beast Over, which EXE6's rules and exe6-compat's
/// `forms` read; EXE5's `souls`, Soul Unison's), and a set's `form_list`
/// the forms it lists, in order.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct NaviForms {
    /// The sets' form lists, by the set's name (a version's: EXE6's Crosses,
    /// which its form list offers a player of that version; EXE5's souls,
    /// which its soul button offers). A frontend numbers a set's pictures
    /// of them by it.
    pub lists: Vec<(String, Vec<FormHandle>)>,
}

impl NaviForms {
    /// The forms the set named `set` lists, in order.
    pub fn listed(&self, set: &str) -> &[FormHandle] {
        self.lists.iter().find(|(v, _)| v == set).map_or(&[], |(_, forms)| forms)
    }

    /// The first of the lists that holds `form`, with the form's place in
    /// it (from 0).
    pub fn holding(&self, form: FormHandle) -> Option<(usize, &[FormHandle])> {
        self.lists.iter().find_map(|(_, forms)| Some((forms.iter().position(|&f| f == form)?, forms.as_slice())))
    }
}

/// A navi's stats when a navi switch brings it fresh (`byte_80210DD`,
/// with its weapons): its HP, its body's programs, the first barrier, its
/// Mega and Giga levels, and the damage of its B+Back special.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct FreshStats {
    pub hp: u16,
    pub super_armor: bool,
    pub float_shoes: bool,
    pub air_shoes: bool,
    pub undershirt: bool,
    pub first_barrier: Option<nettai_content_api::RecordHandle>,
    pub mega_level: u8,
    pub giga_level: u8,
    pub back_special_damage: u16,
}

/// A link navi's damage bonus on the damaging chips of its family
/// (`sub_800F09E`): how much, a function of its side (`damage`, which the
/// round's setup asks: EXE6's by the navi's level, `byte_8021300`, 15 a
/// navi).
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NaviChipBonus {
    pub family: ChipFamily,
    /// Dimming chips of the family count too.
    #[serde(default)]
    pub dimming_chips: bool,
}

/// The chips a navi charges with A: a family's, but never its own chip;
/// only where its `when`, a function of its side, says so (EXE6's from a
/// navi level), else always.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NaviChargedChips {
    pub family: ChipFamily,
    /// Only its damaging chips that aren't dimming chips (false: any). A
    /// rule states it: there is no usual answer.
    pub damaging: bool,
    /// Only its chips that are neither dimming chips nor dark chips.
    #[serde(default)]
    pub plain: bool,
}

/// The chips a rule is about: an element's, or a family's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ChipMatch {
    Element(Element),
    Family(ChipFamily),
}

/// One of MegaMan's forms. (What a game's rules say of their game's
/// forms, EXE6's kinds of form among it, is their extension:
/// `SystemDef::extends`.)
#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FormData {
    /// Its game's base form: what the game's navis are in before they
    /// change form, and what a revert takes them back to (one a game).
    #[serde(default)]
    pub base: bool,
    pub sprite: SpriteId,
    pub element: Element,
    #[serde(default)]
    pub weakness: SecondaryElements,
    /// Added to the buster's damage.
    pub buster_bonus: u8,
    /// MegaMan's palette in it (`byte_80203EA`: a Cross's; the base form's
    /// follows the mood and the style, a form's with `mood_palette` the
    /// mood).
    #[serde(default)]
    pub palette: u8,
    /// Its faces in the emotion window (`sub_801E6A8`), by emotion.
    /// Presentation only.
    #[serde(default, deserialize_with = "faces")]
    pub mugshot: Option<Faces>,
    /// The version of its game the form belongs to (EXE6's Gregar's or
    /// Falzar's Crosses and Beast), by the name its game's pack keeps a
    /// version's pictures under; none: every version's. Its game's rules
    /// may read it (EXE6's: a Beast's roar, the Crosses a Beast goes with);
    /// to the engine it is presentation only (whose pictures name it in its
    /// window and show its Beast, which console has its face).
    #[serde(default)]
    pub version: Option<String>,
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
    /// target marker).
    #[serde(default)]
    pub status_reset: FormEffects,
    #[serde(default)]
    pub ability_refresh: Option<FormEffects>,
    /// The height it floats at, in whole pixels (`sub_80F0608`).
    #[serde(default)]
    pub hover: i16,
    /// Its glow: the navi's color shader by the battle time, one entry a
    /// tick, over and over (`sub_8016A38`: Beast Over's, `byte_8016A68` and
    /// `byte_8016A9C`). A navi in it takes no sprite palette
    /// (`sub_80100EC`) and no invulnerable glow (`sub_8016860`).
    #[serde(default)]
    pub glow: Option<Vec<u16>>,
    /// The lag at the end of a move in it, in place of MegaMan's 4 (EXE5's
    /// ShadowSoul's 0: 0x0800E0D2).
    #[serde(default)]
    pub move_lag: Option<u8>,
    /// What primes it, and what a primed use doubles (EXE5's GyroSoul).
    #[serde(default)]
    pub priming: Option<Priming>,
    /// The damaging chips (not dimming chips) that deal double while it
    /// stands on grass, which the use turns normal (EXE5's TomahawkSoul's
    /// Wood chips: 0x0801032A).
    #[serde(default)]
    pub grass_doubles: Option<ChipMatch>,
    /// The ticks a damaging chip (not a dimming chip) used with the panel
    /// ahead not its side's keeps it invulnerable (EXE5's KnightSoul's 50:
    /// 0x08010392).
    #[serde(default)]
    pub front_guard: Option<u16>,
    #[serde(default)]
    pub traits: FormTraits,
    /// (This and what follows are read from the definition by handle, not
    /// with the rest of the record.)
    /// The action that changes a navi into it (a game's form change: EXE6's
    /// forms', content/exe6/rules/forms), which the pause handler runs at a
    /// turn's start.
    #[serde(skip)]
    pub change: Option<nettai_content_api::ActionHandle>,
    /// The action that takes a navi out of it back to its base form when
    /// its side asks (EXE5's souls': 0x080121D8), run while paused; none:
    /// the framework's revert (`sub_8015614`, EXE6's forms).
    #[serde(skip)]
    pub revert: Option<nettai_content_api::ActionHandle>,
    /// The action its charged chips start in place of their own (EXE5's
    /// NapalmSoul's, action 0x4B: 0x08010442).
    #[serde(skip)]
    pub charged_action: Option<nettai_content_api::ActionHandle>,
    /// What a weakness hit drops it to (`sub_8015766`); none: it stays.
    #[serde(skip)]
    pub breaks_to: Option<FormHandle>,
    #[serde(skip)]
    pub weapons: FormWeapons,
    /// Its identity (the base form has none of its own: the navi's).
    #[serde(skip)]
    pub identity: Option<IdentityHandle>,
}

/// A form's priming (EXE5's GyroSoul: AIData +0x0D): the use of a chip it
/// is primed by (not a dimming chip) primes it, with a sound (0x080102D2);
/// primed, the next damaging chip it doubles (not a dimming chip) deals
/// double and spends it, and nothing else doubles a chip meanwhile: not
/// Full Synchro, not anger (0x0801026C, 0x08010302). The status reset
/// spends it.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Priming {
    pub by: ChipMatch,
    pub sound: crate::sound::SoundId,
    pub doubles: Vec<ChipMatch>,
}

/// A form's faces in the emotion window, by its navi's emotion
/// (`sub_8015B54`): the mugshots' numbers. (In EXE6 the base form has
/// MegaMan's five, a Cross a tired one besides, a Beast a Full Synchro one:
/// `sub_801E6A8` adds 5 or 1 to `byte_801E700`'s picture. EXE5's MegaMan
/// has a worried one besides, and a second set: his Hub Style's,
/// 0x0801AF64.)
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize)]
pub struct Faces {
    pub own: FaceSet,
    /// The second set, which the side's rules may show instead
    /// (`SideLooks::face_variant`, `face_variant_charged`).
    pub variant: Option<FaceSet>,
}

/// One set of faces, keyed by its game's emotions' names (the rules'
/// `emotion` order's: `Content::define` holds a set to them). An emotion
/// without a face of its own shows the set's one face, else its game's
/// default emotion's (`Emotion(0)`'s: EXE6's and EXE5's normal).
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, Serialize)]
pub struct FaceSet {
    /// The faces by their emotion's name, in name order (a table's).
    pub by_emotion: Vec<(String, super::MugshotId)>,
    /// The face of every emotion (a definition's one face, `mugshot = ...`).
    pub every: Option<super::MugshotId>,
}

impl FaceSet {
    /// The face for emotion `emotion` of its game's (`emotions`): its own,
    /// else the set's one face, else the default emotion's.
    pub fn of(&self, emotions: &super::EmotionRules, emotion: super::Emotion) -> super::MugshotId {
        let named = |name: &str| self.by_emotion.iter().find(|(n, _)| n == name).map(|&(_, face)| face);
        named(emotions.name(emotion))
            .or(self.every)
            .or_else(|| named(emotions.name(super::Emotion::default())))
            .unwrap_or_default()
    }

    /// Why the set doesn't fit its game's emotions (`emotions`): a name
    /// that isn't one of them, or no face for an emotion without one of its
    /// own.
    fn check(&self, emotions: &super::EmotionRules) -> Result<(), String> {
        if let Some((name, _)) = self.by_emotion.iter().find(|(n, _)| emotions.by_name(n).is_none()) {
            return Err(format!("{name:?} is none of its game's emotions ({})", emotions.names.join(", ")));
        }
        let default = emotions.name(super::Emotion::default());
        if self.every.is_none() && !self.by_emotion.iter().any(|(n, _)| n == default) {
            return Err(format!("no face for {default:?}, the face of an emotion without one of its own"));
        }
        Ok(())
    }
}

impl Faces {
    /// The face for `emotion` in the set the side shows: the second set's
    /// when `variant` and the form has one.
    pub fn shown(&self, emotions: &super::EmotionRules, emotion: super::Emotion, variant: bool) -> super::MugshotId {
        match &self.variant {
            Some(v) if variant => v.of(emotions, emotion),
            _ => self.own.of(emotions, emotion),
        }
    }
}

/// Every form's faces fit its game's emotions (the rules' `emotion`): each
/// set's names are the game's emotions', and an emotion without a face
/// of its own has one to show.
pub(crate) fn check_faces(content: &super::Content) -> Result<(), nettai_content_api::ContentError> {
    let Some(rules) = &content.rules else { return Ok(()) };
    for d in &content.defs.forms {
        let Some(faces) = &d.record.mugshot else { continue };
        for (set, at) in [(Some(&faces.own), "mugshot"), (faces.variant.as_ref(), "mugshot.variant")] {
            if let Some(Err(m)) = set.map(|s| s.check(&rules.emotion)) {
                return Err(nettai_content_api::ContentError::new(format!("form {}: {at}: {m}", d.key)));
            }
        }
    }
    Ok(())
}

/// A form definition's `mugshot`: one face, or faces by emotion name (and
/// a second set, `variant`).
fn faces<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Option<Faces>, D::Error> {
    use serde::de::Error;
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Entry {
        Face(super::MugshotId),
        Set(std::collections::BTreeMap<String, super::MugshotId>),
    }
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Spec {
        One(super::MugshotId),
        ByEmotion(std::collections::BTreeMap<String, Entry>),
    }
    let Some(spec) = Option::<Spec>::deserialize(d)? else { return Ok(None) };
    let entries = match spec {
        Spec::One(face) => return Ok(Some(Faces { own: FaceSet { every: Some(face), ..FaceSet::default() }, variant: None })),
        Spec::ByEmotion(entries) => entries,
    };
    let mut faces = Faces::default();
    for (name, entry) in entries {
        match (name.as_str(), entry) {
            ("variant", Entry::Set(set)) => faces.variant = Some(FaceSet { by_emotion: set.into_iter().collect(), every: None }),
            ("variant", Entry::Face(_)) => return Err(D::Error::custom("`variant` is a set of faces by emotion")),
            (_, Entry::Face(face)) => faces.own.by_emotion.push((name, face)),
            (_, Entry::Set(_)) => return Err(D::Error::custom(format!("`{name}` is a face, not a set (only `variant` is)"))),
        }
    }
    Ok(Some(faces))
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
        self.ability_refresh.unwrap_or(FormEffects(self.status_reset.0 & !FormEffects::TARGET_MARKER))
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
    /// Only on a use that isn't charged, with the A charge not full (EXE5's
    /// NapalmSoul, 0x0800D0A6).
    #[serde(default)]
    pub uncharged: bool,
}

/// A navi's charge glow's lift (`NaviData::charge_glow_lift`): the
/// animations of the navi it holds in, and its height in whole pixels.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChargeGlowLift {
    pub anims: Vec<u8>,
    pub pixels: i32,
}

/// The damage a navi adds to a family's damaging chips used standing on a
/// type of panel (`NaviData::panel_bonus`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PanelChipBonus {
    pub panel: crate::field::PanelType,
    pub family: ChipFamily,
    pub damage: u16,
}

/// Chips a form charges with A: a family's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChargedChips {
    pub family: ChipFamily,
    /// Only its damaging chips that aren't dimming chips (false: any). A
    /// rule states it, as a navi's does: there is no usual answer.
    pub damaging: bool,
    /// And the chips with the `element_sword` trait.
    #[serde(default)]
    pub element_swords: bool,
    /// Only its chips that are neither dimming chips nor dark chips (EXE5's
    /// souls, 0x0801090A).
    #[serde(default)]
    pub plain: bool,
}

/// What a charged chip gains in a form, or for a navi.
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
    /// The target marker (EXE6's Beast forms' lock-on marker).
    pub const TARGET_MARKER: u16 = 0x040;
    /// Invulnerable for good.
    pub const INVULNERABLE: u16 = 0x080;
    /// The navi's controller starts over (EXE6's Beast Over's berserk).
    pub const CONTROLLER_RESTART: u16 = 0x100;
    pub(crate) const NAMES: &[(u32, &str)] = &[
        (0x001, "clear_statuses"),
        (0x002, "super_armor"),
        (0x004, "air_shoes"),
        (0x008, "float_shoes"),
        (0x010, "floating_body"),
        (0x020, "untouchable"),
        (0x040, "target_marker"),
        (0x080, "invulnerable"),
        (0x100, "controller_restart"),
    ];

    pub fn has(self, bit: u16) -> bool {
        self.0 & bit != 0
    }
}

serde_flags!(FormEffects, u16);

/// What the rules ask of particular forms. In a content file, a list of
/// names.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct FormTraits(pub u16);

impl FormTraits {
    /// A hit's status doesn't take (TomahawkCross).
    pub const STATUS_IMMUNE: u16 = 0x001;
    /// Its damaging Null chips delete at once what has a 4 in its HP
    /// (`sub_8012C4A`: EraseCross).
    pub const ERASES: u16 = 0x002;
    /// Its held buster doesn't fire while its B+Back special is asked for
    /// (TenguCross and DustCross in Beast Out).
    pub const SPECIAL_HOLDS_BUSTER: u16 = 0x004;
    /// A metal panel doesn't slide the navi (EXE5's MagnetSoul:
    /// 0x08017216).
    pub const STANDS_ON_METAL: u16 = 0x008;
    /// The side's rules' controller decides the navi's idle (Beast Over's
    /// berserk, `sub_802D322`): the player's buttons don't reach it
    /// (`apply_actor_inputs`), and a full gauge opens the custom screen.
    pub const CONTROLLED: u16 = 0x010;
    /// The navi's target marker shows (`sub_80E1566`: EXE6's Beast forms,
    /// 0x0B to 0x18).
    pub const SHOWS_TARGET_MARKER: u16 = 0x020;
    /// An afterimage of the navi lasts while it stays in a form with this
    /// trait, not until its attack ends (`sub_80E341E`: EXE6's Beast forms).
    pub const AFTERIMAGES_STAY: u16 = 0x040;
    /// With a Null chip next, its A charge's time is the alternative
    /// A-charge routine's (`sub_8012F62`: EXE6's Beast forms).
    pub const ALT_CHARGE_TIME: u16 = 0x080;
    /// Its damaging Null chips deal double, outside battle mode 1
    /// (`sub_8012ABC`: Beast Over).
    pub const DOUBLES_NULL: u16 = 0x100;
    /// Its palette follows the mood: 4 in Full Synchro (mood 0xFF), else
    /// 0, in place of its `palette` (`sub_80100EC`: Beast Out's).
    pub const MOOD_PALETTE: u16 = 0x200;
    pub(crate) const NAMES: &[(u32, &str)] = &[
        (0x001, "status_immune"),
        (0x002, "erases"),
        (0x004, "special_holds_buster"),
        (0x008, "stands_on_metal"),
        (0x010, "controlled"),
        (0x020, "shows_target_marker"),
        (0x040, "afterimages_stay"),
        (0x080, "alt_charge_time"),
        (0x100, "doubles_null"),
        (0x200, "mood_palette"),
    ];

    pub fn has(self, bit: u16) -> bool {
        self.0 & bit != 0
    }
}

serde_flags!(FormTraits, u16);

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
    // (Its functions of its side are `NaviDef::given`'s.)
    let mut d = d.clone();
    for (table, field) in [("chip_bonus", "damage"), ("charged_chips", "when")] {
        if let nettai_content_api::Data::Map(entries) = &mut d.spec {
            for (k, v) in entries.iter_mut() {
                if k.to_string() == table {
                    super::reader::strip(v, &[field]);
                }
            }
        }
    }
    let d = &d;
    let mut o = super::reader::fields(
        d,
        r,
        &[
            "id", "identity", "banners", "own_chip", "actions", "weapons", "fresh", "switch_hp", "levels", "story", "forms", "tick",
            "idle", "post_init", "fire_charge",
        ],
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

/// A form definition's record, likewise, past `extended`: the fields its
/// game's rules extend forms with (theirs to check, `SystemDef::extends`).
pub(crate) fn read_form(
    d: &nettai_content_api::Definition,
    r: &super::reader::SpecReader,
    extended: &[&str],
) -> Result<FormData, nettai_content_api::ContentError> {
    use serde_json::Value as Json;
    // (`buster_arm` is the content's own: the arm a navi raises.)
    let own = [
        "id",
        "identity",
        "breaks_to",
        "change",
        "revert",
        "charged_action",
        "weapons",
        "buster_arm",
        "reset",
        "put_on",
        "take_off",
        "tick",
    ];
    let skip: Vec<&str> = own.iter().chain(extended).copied().collect();
    let o = super::reader::fields(d, r, &skip)?;
    let form: FormData = serde_json::from_value(Json::Object(o)).map_err(|m| super::reader::err(d, m))?;
    Ok(form)
}

/// A navi definition's `fresh`: what a navi switch brings it with.
pub(crate) fn read_fresh(
    d: &nettai_content_api::Definition,
    record: impl Fn(&str) -> Option<nettai_content_api::RecordHandle>,
) -> Result<Option<FreshStats>, nettai_content_api::ContentError> {
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
        first_barrier: match fresh.field("first_barrier") {
            Data::Nil => None,
            Data::Ref(nettai_content_api::Registry::Record, key) => {
                Some(record(key).ok_or_else(|| what(format!("fresh.first_barrier names no record {key:?}")))?)
            }
            other => return Err(what(format!("fresh.first_barrier is {other:?}, not a barrier (lib/barriers)"))),
        },
        mega_level: number("mega_level", 0xFF)? as u8,
        giga_level: number("giga_level", 0xFF)? as u8,
        back_special_damage: match fresh.field("back_special_damage") {
            Data::Nil => 0,
            _ => number("back_special_damage", 0xFFFF)? as u16,
        },
    }))
}

/// A navi definition's `switch_hp`: its HP after a navi switch, by side.
pub(crate) fn read_switch_hp(d: &nettai_content_api::Definition) -> Result<Option<[u16; 2]>, nettai_content_api::ContentError> {
    use nettai_content_api::{ContentError, Data};
    let what = || ContentError::new(format!("{}.luau: navi {}: `switch_hp` is two HP values, by side", d.module, d.key));
    match d.spec.field("switch_hp") {
        Data::Nil => Ok(None),
        Data::List(items) => match items.as_slice() {
            [Data::Int(a), Data::Int(b)] if (0..=0xFFFF).contains(a) && (0..=0xFFFF).contains(b) => Ok(Some([*a as u16, *b as u16])),
            _ => Err(what()),
        },
        _ => Err(what()),
    }
}

/// A navi definition's `story`: its HP by the story's progress and its
/// last level.
pub(crate) fn read_story(d: &nettai_content_api::Definition) -> Result<Option<NaviStory>, nettai_content_api::ContentError> {
    use nettai_content_api::{ContentError, Data};
    let what = |m: String| ContentError::new(format!("{}.luau: navi {}: {m}", d.module, d.key));
    let story = match d.spec.field("story") {
        Data::Nil => return Ok(None),
        v @ Data::Map(entries) => {
            if let Some((k, _)) = entries.iter().find(|(k, _)| !["hp", "max_level"].contains(&k.to_string().as_str())) {
                return Err(what(format!("story has no field `{k}` (it has hp, max_level)")));
            }
            v
        }
        other => return Err(what(format!("`story` is {other:?}, not a table"))),
    };
    let max_level = match story.field("max_level") {
        Data::Int(i) if (0..0xFF).contains(i) => *i as u8,
        other => return Err(what(format!("story.max_level is {other:?}, not a level (0 to 254)"))),
    };
    let hp = match story.field("hp") {
        Data::List(items) => items
            .iter()
            .enumerate()
            .map(|(i, v)| match v {
                Data::Int(n) if (1..=0xFFFF).contains(n) => Ok(*n as u16),
                other => Err(what(format!("story.hp[{}] is {other:?}, not HP up to 65535", i + 1))),
            })
            .collect::<Result<Vec<u16>, _>>()?,
        other => return Err(what(format!("story.hp is {other:?}, not a list of HP"))),
    };
    // (A level below the last is the progress: each has its entry.)
    if hp.len() <= max_level as usize {
        return Err(what(format!("story.hp has {} entries: levels 0 to {max_level} need {}", hp.len(), max_level as usize + 1)));
    }
    Ok(Some(NaviStory { hp, max_level }))
}

/// A navi definition's `levels`: its base HP by the story's progress and
/// each level's gains, the B+Back special a level sets by the weapon it
/// names.
pub(crate) fn read_levels(
    d: &nettai_content_api::Definition,
    weapon: impl Fn(&str) -> Option<WeaponHandle>,
) -> Result<Option<NaviLevels>, nettai_content_api::ContentError> {
    use nettai_content_api::{ContentError, Data, Registry};
    let what = |m: String| ContentError::new(format!("{}.luau: navi {}: {m}", d.module, d.key));
    let levels = match d.spec.field("levels") {
        Data::Nil => return Ok(None),
        v @ Data::Map(_) => v,
        other => return Err(what(format!("`levels` is {other:?}, not a table"))),
    };
    // (A field the reload has no use for is a mistake.)
    let known = |v: &Data, at: &str, names: &[&str]| -> Result<(), ContentError> {
        if let Data::Map(entries) = v
            && let Some((k, _)) = entries.iter().find(|(k, _)| !names.contains(&k.to_string().as_str()))
        {
            return Err(what(format!("{at} has no field `{k}` (it has {})", names.join(", "))));
        }
        Ok(())
    };
    known(levels, "levels", &["base_hp", "by_level"])?;
    let number = |v: &Data, at: &str, max: i64| -> Result<i64, ContentError> {
        match v {
            Data::Int(i) if (0..=max).contains(i) => Ok(*i),
            other => Err(what(format!("{at} is {other:?}, not a number up to {max}"))),
        }
    };
    let list = |field: &str| -> Result<&[Data], ContentError> {
        match levels.field(field) {
            Data::List(items) if !items.is_empty() => Ok(items),
            other => Err(what(format!("levels.{field} is {other:?}, not a list of one or more"))),
        }
    };
    let base_hp = list("base_hp")?
        .iter()
        .enumerate()
        .map(|(i, v)| number(v, &format!("levels.base_hp[{}]", i + 1), 0xFFFF).map(|n| n as u16))
        .collect::<Result<_, _>>()?;
    let mut by_level = Vec::new();
    for (i, g) in list("by_level")?.iter().enumerate() {
        let at = |field: &str| format!("levels.by_level[{}].{field}", i + 1);
        if !matches!(g, Data::Map(_)) {
            return Err(what(format!("levels.by_level[{}] is {g:?}, not a level's gains", i + 1)));
        }
        known(
            g,
            &format!("levels.by_level[{}]", i + 1),
            &[
                "hp",
                "attack",
                "rapid",
                "charge",
                "custom_level",
                "mega_level",
                "super_armor",
                "float_shoes",
                "air_shoes",
                "back_special",
            ],
        )?;
        let small = |field: &str| -> Result<u8, ContentError> {
            match g.field(field) {
                Data::Nil => Ok(0),
                v => number(v, &at(field), 0xFF).map(|n| n as u8),
            }
        };
        let flag = |field: &str| -> Result<bool, ContentError> {
            match g.field(field) {
                Data::Nil => Ok(false),
                Data::Bool(b) => Ok(*b),
                other => Err(what(format!("{} is {other:?}, not true or false", at(field)))),
            }
        };
        by_level.push(LevelGain {
            hp: number(g.field("hp"), &at("hp"), 0xFFFF)? as u16,
            attack: small("attack")?,
            rapid: small("rapid")?,
            charge: small("charge")?,
            custom_level: small("custom_level")?,
            mega_level: small("mega_level")?,
            super_armor: flag("super_armor")?,
            float_shoes: flag("float_shoes")?,
            air_shoes: flag("air_shoes")?,
            back_special: match g.field("back_special") {
                Data::Nil => None,
                Data::Ref(Registry::Weapon, key) => {
                    Some(weapon(key).ok_or_else(|| what(format!("{} names {key:?}, which is not a weapon", at("back_special"))))?)
                }
                other => return Err(what(format!("{} is {other:?}, not a weapon", at("back_special")))),
            },
        });
    }
    Ok(Some(NaviLevels { base_hp, by_level }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_form_shows_its_face_for_an_emotion_or_its_default_ones() {
        #[derive(Deserialize)]
        struct Form {
            #[serde(default, deserialize_with = "faces")]
            mugshot: Option<Faces>,
        }
        let read = |json: &str| serde_json::from_str::<Form>(json).unwrap().mugshot;
        // (A game's emotions: the last case's, the default, is normal.)
        use super::super::{AngerEnd, EmotionRules, EmotionWhen, MoodHeld};
        let when = || vec![EmotionWhen { angry: Some(true), ..Default::default() }];
        let mut cases: Vec<_> = ["tired", "angry", "full_synchro", "worried"].map(|n| (n.to_string(), when())).to_vec();
        cases.push(("normal".to_string(), Vec::new()));
        let emotions = EmotionRules::new(MoodHeld::AtZero, AngerEnd::ThroughSetter, cases, Vec::new()).unwrap();
        let e = |name: &str| emotions.by_name(name).unwrap();
        // One face, whatever the emotion.
        let one = read(r#"{ "mugshot": 15 }"#).unwrap();
        assert_eq!(["normal", "tired", "full_synchro"].map(|n| one.shown(&emotions, e(n), false).0), [15; 3]);
        // A Cross's: its own, and a tired one.
        let cross = read(r#"{ "mugshot": { "normal": 5, "tired": 10 } }"#).unwrap();
        assert_eq!(["normal", "angry", "tired"].map(|n| cross.shown(&emotions, e(n), false).0), [5, 5, 10]);
        assert_eq!(read("{}"), None);
        // EXE5's MegaMan: a worried face, and a second set the side may show.
        let exe5 = read(r#"{ "mugshot": { "normal": 0, "worried": 2, "variant": { "normal": 11, "worried": 13 } } }"#).unwrap();
        assert_eq!(["worried", "angry"].map(|n| exe5.shown(&emotions, e(n), false).0), [2, 0]);
        assert_eq!(["worried", "angry"].map(|n| exe5.shown(&emotions, e(n), true).0), [13, 11]);
        assert_eq!(cross.shown(&emotions, e("tired"), true).0, 10);
        // A set holds to its game's emotions: their names, and a face for
        // the default (the first's).
        assert!(exe5.own.check(&emotions).is_ok() && one.own.check(&emotions).is_ok());
        let wrong = read(r#"{ "mugshot": { "normal": 0, "dark": 2 } }"#).unwrap();
        assert!(wrong.own.check(&emotions).unwrap_err().contains("\"dark\" is none of its game's emotions"));
        let defaultless = read(r#"{ "mugshot": { "tired": 2 } }"#).unwrap();
        assert!(defaultless.own.check(&emotions).unwrap_err().contains("no face for \"normal\""));
    }

    /// Families a game might have, as its definitions are read with them
    /// (`super::super::reading_families`).
    fn families() -> super::super::ChipFamilies {
        let families = ["wood", "sword", "null"].iter().enumerate().map(|(i, n)| (n.to_string(), ChipFamily(i as u8))).collect();
        super::super::ChipFamilies { families, non_elemental: ChipFamily(2) }
    }

    #[test]
    fn a_navis_charged_chips_say_whether_they_must_be_damaging() {
        let read = |json: &str| super::super::reading_families(&families(), || serde_json::from_str::<NaviChargedChips>(json));
        // EXE6's link navis: their family's damaging chips (from a level:
        // their `when`, a function of the side).
        let exe6 = read(r#"{ "family": "wood", "damaging": true }"#).unwrap();
        assert_eq!((exe6.damaging, exe6.plain), (true, false));
        // EXE5's team navis: any chip of the family that is neither a
        // dimming nor a dark chip, at any level.
        let exe5 = read(r#"{ "family": "wood", "damaging": false, "plain": true }"#).unwrap();
        assert_eq!((exe5.damaging, exe5.plain), (false, true));
        // Left out, the rule doesn't load, and the error names the field.
        let error = read(r#"{ "family": "wood" }"#).unwrap_err().to_string();
        assert!(error.contains("missing field `damaging`"), "{error}");
    }

    #[test]
    fn a_forms_charged_chips_say_whether_they_must_be_damaging() {
        let read = |json: &str| super::super::reading_families(&families(), || serde_json::from_str::<ChargedChips>(json));
        // A Cross's: its family's damaging chips (SlashCross's the element
        // swords too); Beast Out's any Null chip.
        let cross = read(r#"{ "family": "sword", "damaging": true, "element_swords": true }"#).unwrap();
        assert_eq!((cross.damaging, cross.element_swords, cross.plain), (true, true, false));
        let beast = read(r#"{ "family": "null", "damaging": false }"#).unwrap();
        assert_eq!((beast.damaging, beast.element_swords, beast.plain), (false, false, false));
        // Left out, the rule doesn't load, and the error names the field.
        let error = read(r#"{ "family": "wood" }"#).unwrap_err().to_string();
        assert!(error.contains("missing field `damaging`"), "{error}");
        // A family is one of its game's.
        let error = read(r#"{ "family": "metal", "damaging": true }"#).unwrap_err().to_string();
        assert!(error.contains("\"metal\" is none of its game's chip families (wood, sword, null)"), "{error}");
    }
}
