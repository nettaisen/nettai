//! Game content, extracted from the original game by `bn6-extract` into
//! typed tables. The files named `*_generated.rs` are written by that tool;
//! regenerate them rather than editing by hand.

mod actor_lists_generated;
pub mod attacks;
mod attacks_generated;
mod banners_generated;
mod chips_generated;
pub mod collision_generated;
pub mod effects_generated;
mod obstacles_generated;
mod sprites_generated;
pub mod field_generated;
pub mod lockon;
mod lockon_generated;
pub mod player;
pub mod player_generated;

pub use actor_lists_generated::ACTOR_LISTS;
pub use banners_generated::{BANNER_HOLDS, LOSE_BANNERS, WIN_BANNERS};
pub use chips_generated::CHIPS;
pub use obstacles_generated::{ABSORBED_SPRITES, ROCKS};

/// A kind of rock (one row of `byte_80CF934`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RockKind {
    /// The rock's standing animation.
    pub anim: u8,
    pub hp: u16,
    pub element: Element,
    /// Palette of the debris it breaks into.
    pub debris_palette: u8,
    /// Sound it breaks with.
    pub break_sound: u16,
    pub name_id: u16,
}

/// Chip ids are indices into [`CHIPS`] (0..=0x19A).
pub type ChipId = u16;

/// A chip code: A-Z are 0-25, `*` is 26.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ChipCode(pub u8);

impl ChipCode {
    pub const ASTERISK: ChipCode = ChipCode(26);

    pub fn letter(self) -> char {
        if self.0 == 26 { '*' } else { (b'A' + self.0) as char }
    }
}

/// An attack's primary element.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Element {
    Null = 0,
    Fire = 1,
    Aqua = 2,
    Elec = 3,
    Wood = 4,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChipClass {
    Standard,
    Mega,
    Giga,
    /// Not a folder chip (cross/beast attacks, internal chips).
    Special,
    ProgramAdvance,
}

/// Chip record flags (+0x09).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ChipFlags(pub u8);

impl ChipFlags {
    /// Stops time when used; can counter during time stop.
    pub const TIME_FREEZE: u8 = 0x01;
    /// Deals damage: shown on the banner, boostable by Atk+ and forms.
    pub const HAS_DAMAGE: u8 = 0x02;
    /// Navi chip: boosted by Navi+.
    pub const NAVI: u8 = 0x04;
    /// Damage recomputed every tick while this is the next chip.
    pub const VARIABLE_DAMAGE: u8 = 0x80;

    pub fn has(self, bit: u8) -> bool {
        self.0 & bit != 0
    }
}

/// One battle chip's data record (see docs/engine/chips.md §1.2).
#[derive(Clone, Copy, Debug)]
pub struct ChipData {
    pub name: &'static str,
    /// Codes the chip comes in (up to four).
    pub codes: &'static [ChipCode],
    pub element: Element,
    /// Stars minus one.
    pub rarity: u8,
    /// Icon family; Sword/Cursor/Wind/Break also give secondary elements.
    pub family: u8,
    pub class: ChipClass,
    /// Folder memory cost.
    pub mb: u8,
    pub flags: ChipFlags,
    /// Counter/stagger strength carried to the attack's hitbox.
    pub hit_param: u8,
    /// The attack action the user performs.
    pub action: u8,
    /// Variant within the action (e.g. Cannon/HiCannon/M-Cannon = 0/1/2).
    /// (Record bytes +0x0D and +0x0E have no reader and are not
    /// extracted.)
    pub subtype: u8,
    /// Beast Out auto-lock-on.
    pub beast_lockon: u8,
    /// Action-specific parameters.
    pub params: u32,
    /// Input lockout after the attack ends, in ticks.
    pub lockout: u8,
    pub lib_index: u8,
    pub flags2: u8,
    /// Beast Out lock-on panel selector.
    pub lockon_mode: u8,
    pub sort_key: u16,
    /// Base damage; values of 1000 and up select a damage formula.
    pub damage: u16,
    pub library_no: u16,
    pub slotin_max: u8,
    /// Dark chip substitute when the user has no bug frags (0xFF = none).
    pub dark_subst: u8,
}

/// The data record for a chip.
pub fn chip(id: ChipId) -> &'static ChipData {
    &CHIPS[id as usize]
}

/// A sprite: (category byte offset, index) into the game's sprite table.
pub use bn6_content_api::SpriteId;

/// One animation frame's timing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AnimFrame {
    /// Ticks the frame shows for.
    pub duration: u8,
    /// Frame cue bits: `object::sprite::FRAME_LAST` (0x80) ends the
    /// animation, `FRAME_LOOP` (0x40) loops it; attacks read others as cues.
    pub flags: u8,
}

/// A sprite's animations, each a list of frames.
#[derive(Clone, Copy, Debug)]
pub struct SpriteAnimations {
    pub id: SpriteId,
    pub animations: &'static [&'static [AnimFrame]],
}

/// An animation's frames (empty when the sprite has no battle animation
/// data).
pub fn animation(sprite: SpriteId, anim: u8) -> &'static [AnimFrame] {
    let key = |s: &SpriteAnimations| (s.id.category, s.id.index);
    match sprites_generated::SPRITES.binary_search_by_key(&(sprite.category, sprite.index), key) {
        Ok(i) => sprites_generated::SPRITES[i].animations.get(anim as usize).copied().unwrap_or(&[]),
        Err(_) => &[],
    }
}

/// A one-shot effect's look: which sprite animation it plays.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EffectSprite {
    pub sprite: SpriteId,
    pub anim: u8,
    pub palette: u8,
}

/// A HUD banner (the game's UI banner id).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BannerId(pub u8);

impl BannerId {
    /// Whether the banner stays up until removed rather than timing out.
    pub fn holds(self) -> bool {
        BANNER_HOLDS[(self.0 / 4) as usize]
    }
}

/// A panel relative to another, `dx` toward the facing side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PanelOffset {
    pub dx: i8,
    pub dy: i8,
}

/// A test on a panel's flags: all of `require` set and none of `forbid`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PanelCondition {
    pub require: u32,
    pub forbid: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Total ticks of an effect's animation (its lifetime when not timed).
    fn effect_ticks(id: usize) -> u32 {
        let e = effects_generated::EFFECTS[id];
        animation(e.sprite, e.anim).iter().map(|f| f.duration as u32).sum()
    }

    #[test]
    fn effect_lifetimes_match_the_game() {
        // Measured in the original game (objects-and-player.md §A.3).
        assert_eq!(effect_ticks(0x03), 22);
        assert_eq!(effect_ticks(0x39), 11);
        assert_eq!(effect_ticks(0x3A), 11);
    }
}
