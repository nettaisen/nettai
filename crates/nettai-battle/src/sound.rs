//! Sound output. The engine plays no sound; each tick it reports the sound
//! calls the original makes at that point as typed cues, which a frontend
//! plays (nettai-audio does, with the game's own sound driver and data). Cues
//! are output only: nothing in the simulation reads them.
//!
//! A few calls the original makes on one console only (a player's own
//! charge and hit sounds, their pinch and result music); the engine records
//! what each side's player hears (`Battle::sound_cues_for`). Under rollback
//! a tick can run more than once: `cues` plays each cue once.
//! See docs/engine/audio.md and docs/design/rollback.md.

/// A sound asset as the pack identifies it (for BN6's pack, an entry of
/// the game's song table, which music and sound effects share). The engine
/// names none itself: content does (`asset.sound`), and what the ruleset
/// plays it gets by role (`Roles::sound`, `Roles::music`) or from a
/// definition (a stage's music, a panel type's trail sound).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SoundId(pub u16);

/// A sound call of the original, as the engine reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SoundCue {
    /// `PlaySoundEffect(id)`: start a song-table entry on its music player
    /// (in practice, sound effects).
    Effect(SoundId),
    /// `PlayMusic(id)`: switch the background music, unless it already is
    /// `id`.
    Music(SoundId),
    /// `musicGameState_8000784`: stop all sound.
    StopMusic,
    /// `sub_8009158`: the local navi's HP fell to a quarter or less (`true`)
    /// or rose above it again (`false`). While it's low the music plays a
    /// semitone higher and 282/256 as fast.
    Pinch(bool),
    /// `sub_802A3CC`: the custom screen closed; any volume change it made to
    /// the music is undone.
    RestoreVolume,
}

impl From<SoundId> for SoundCue {
    fn from(id: SoundId) -> SoundCue {
        SoundCue::Effect(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::battle::{Battle, TickEvents};
    use crate::input::PlayerTick;
    use crate::content::testing;
    use crate::setup::effects;

    /// A battle between two navis with 500 HP on the test content, a link
    /// battle or not, with this music.
    fn battle(music: Option<SoundId>, link: bool) -> Battle {
        let content = testing::restaged(testing::LINK_BATTLE_SIDE0_FIRST, |s| s.music = music);
        let mut setup = testing::round_setup(testing::LINK_BATTLE_SIDE0_FIRST, testing::stats(500));
        setup.content = content.hash();
        setup.settings.effects = if link { effects::LINK } else { 0 };
        setup.rng = 0x1234_5678;
        Battle::new(setup, std::sync::Arc::new(content))
    }

    fn tick(b: &mut Battle) -> Vec<SoundCue> {
        b.tick(&[PlayerTick::default(), PlayerTick::default()], TickEvents::default());
        b.sound_cues().to_vec()
    }

    #[test]
    fn a_link_battle_starts_its_music_and_the_pinch_latch_settles() {
        let mut b = battle(Some(SoundId(0x16)), true);
        // The navi has no HP until its own init runs, later in the tick:
        // the low-HP switch fires for one tick, as in the game.
        let link_battle = b.content.defs.roles.music(crate::content::MusicRole::LinkBattle);
        assert_eq!(tick(&mut b), [SoundCue::Music(link_battle), SoundCue::Pinch(true)]);
        // The other side's player hears the same: its navi's latch too.
        assert_eq!(b.sound_cues_for(1), [SoundCue::Music(link_battle), SoundCue::Pinch(true)]);
        assert_eq!(tick(&mut b), [SoundCue::Pinch(false)]);
        assert_eq!(tick(&mut b), [], "cues last one tick");
    }

    #[test]
    fn other_battles_play_their_settings_music_unless_none() {
        let mut b = battle(Some(SoundId(0x16)), false);
        assert_eq!(tick(&mut b), [SoundCue::Music(SoundId(0x16))]);
        let mut b = battle(None, false);
        assert_eq!(tick(&mut b), []);
    }

    #[test]
    fn cues_do_not_change_the_simulation() {
        let (mut a, mut b) = (battle(Some(SoundId(0x15)), true), battle(Some(SoundId(0x15)), true));
        for _ in 0..40 {
            tick(&mut a);
            b.play_sound(SoundId(0x94));
            tick(&mut b);
            assert_eq!((a.rng.state, a.round.ticks, a.round.frames), (b.rng.state, b.round.ticks, b.round.frames));
        }
    }
}
