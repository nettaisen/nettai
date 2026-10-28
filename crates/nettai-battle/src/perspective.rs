//! What each player is shown when two netplay peers present one
//! simulation.
//!
//! Both peers simulate from one perspective, `RoundSetup::local_side`
//! (shared setup), and each presents the result for its own player, the
//! *viewer*. Where the original keeps per-console state, the engine either
//! keeps it for both sides or derives it per viewer here:
//!
//! - sound: `Battle::sound_cues_for` (the engine records what each side
//!   hears: its charge sounds, its hit sound, its pinch music, its
//!   victory or defeat music);
//! - banners: [`Battle::banner_for`] (the telops say whose chip
//!   it is, the result banner shows the viewer's navi winning or losing)
//!   and [`Battle::telops_for`] (a hidden chip's telop names it only to its
//!   user, if to anyone; where telops run apart from the banners each
//!   console lays its own player's on the banner block) and
//!   [`Battle::used_chip_for`] (the other player's chip, named for a
//!   second); a screen of both consoles (a replay watched from both sides)
//!   names each telop as its user's console does
//!   ([`Battle::telops_for_both`]) and shows each player's own result banner
//!   ([`Battle::result_banner_for`]);
//! - the result: [`Battle::round_end_for`];
//! - what each player sees of the objects: [`Battle::visible_to`] (a blind
//!   player doesn't see the other side's objects, the Beast Out lock-on
//!   marker and the A-button charge glow show only to their owner). Where
//!   the original decides an object's `VISIBLE` flag by its console's
//!   rule, the engine decides it for both viewers ([`Battle::hide_from_blind`],
//!   [`Battle::hide_from_other_side`]) and keeps the local side's in the
//!   flag, which the recordings check;
//! - the custom screen's banner (a Program Advance's):
//!   [`Battle::banner_block_for`], each viewer's own screen's;
//! - how each console draws a sprite ([`Battle::look_for`]): the counter
//!   blink only the player in Full Synchro sees, a mark turned to each
//!   console's own side, a palette by whose panel it is as each console
//!   sees it, the parts a console hides; and the round intro's, where each
//!   console's own navi shows at once and the other side's fades in (the
//!   simulation runs the local side's console's; the other console's view
//!   is drawn from it, `kinds::player::entry`).

use crate::battle::{Battle, BattleResult, RoundEnd, fight};
use crate::object::{ObjectRef, Sight, flags};
use crate::content::{BannerId, Telops};
use crate::setup::SetScore;
use crate::dimming::telop_banner;
use crate::hud::{Banner, Telop, TelopHidden};
use nettai_content_api::ChipHandle;

/// A telop as one player's console shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShownTelop {
    /// It is the other player's chip (the banner on the right).
    pub remote: bool,
    pub name: TelopName,
    /// The damage after the name (0: none).
    pub damage: u16,
    /// "+N" after the damage (0: none).
    pub bonus: u16,
    /// "x2" after the numbers.
    pub doubled: bool,
}

/// The name a telop shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TelopName {
    Chip(ChipHandle),
    /// "????": a hidden chip (the game shows chip 0x171's name).
    Hidden,
    /// The engine wasn't told which chip the telop names.
    Unknown,
}

/// `t` as `viewer`'s console shows it: whose chip it is, and "????" for a
/// chip hidden from that player.
fn shown(t: Telop, viewer: u8) -> ShownTelop {
    let remote = t.side != viewer;
    let hidden = match t.hidden {
        TelopHidden::No => false,
        TelopHidden::FromOpponent => remote,
        TelopHidden::FromBoth => true,
    };
    if hidden {
        return ShownTelop { remote, name: TelopName::Hidden, damage: 0, bonus: 0, doubled: false };
    }
    let name = t.chip.map_or(TelopName::Unknown, TelopName::Chip);
    ShownTelop { remote, name, damage: t.damage, bonus: t.bonus, doubled: t.doubled }
}

impl BattleResult {
    /// The same result for the other side.
    pub fn for_other_side(self) -> BattleResult {
        match self {
            BattleResult::Won => BattleResult::Lost,
            BattleResult::Lost => BattleResult::Won,
            // The rest end the battle for both alike.
            r => r,
        }
    }
}

impl SetScore {
    /// The same score for the other side.
    pub fn for_other_side(self) -> SetScore {
        SetScore { wins: self.losses, losses: self.wins, ..self }
    }
}

impl Battle {
    /// `sub_800EB6C` on `viewer`'s console: its player sees `alliance`'s
    /// objects unless they are the other side's and its navi
    /// (`sub_80103BC`) is blind. A navi whose init hasn't run yet has no
    /// collision data: the game reads its status word through the null
    /// pointer, from the BIOS, which gives the opcode it last fetched (the
    /// game's `missing_collision_status`: EXE6's has the blind bit,
    /// EXE5's hasn't). It happens on a round's first tick when the other
    /// side's navi inits first and its FirstBarrier's visual asks.
    pub fn sees(&self, viewer: u8, alliance: u8) -> bool {
        use crate::collision::f1;
        if alliance & 1 == viewer & 1 {
            return true;
        }
        let Some(navi) = self.player(viewer & 1) else { return true };
        let missing = self.game_rules().missing_collision_status.0;
        let status = self.objects.get(navi).collision.map_or(missing, |c| self.collision.get(c).f1);
        status & f1::BLIND == 0
    }

    /// Whether `viewer`'s player sees object `r`: its `VISIBLE` flag, or
    /// what a rule decided for that viewer ([`Object::sight`]).
    ///
    /// [`Object::sight`]: crate::object::Object::sight
    pub fn visible_to(&self, r: ObjectRef, viewer: u8) -> bool {
        let o = self.objects.get(r);
        match o.sight {
            Sight::Shared => o.flags & flags::VISIBLE != 0,
            Sight::ByViewer(shown) => shown[viewer as usize & 1],
        }
    }

    /// How `viewer`'s console draws `r`'s sprite: its look, or where the
    /// other side's console draws it otherwise, that console's
    /// ([`Look::other`]).
    ///
    /// [`Look::other`]: crate::object::sprite::Look::other
    pub fn look_for(&self, r: ObjectRef, viewer: u8) -> crate::object::sprite::Look {
        let look = &self.objects.sprite(r).look;
        if viewer & 1 == self.round.local_side & 1 { *look } else { look.for_other() }
    }

    /// `r`'s header flags as `viewer`'s console has them: its `VISIBLE`
    /// flag the viewer's (the others are the same for every viewer).
    pub fn flags_for(&self, r: ObjectRef, viewer: u8) -> u8 {
        let shown = if self.visible_to(r, viewer) { flags::VISIBLE } else { 0 };
        self.objects.get(r).flags & !flags::VISIBLE | shown
    }

    /// `r` shown to the viewers `shown` says (by side). Its `VISIBLE` flag
    /// is the local side's.
    pub fn set_visible_by_viewer(&mut self, r: ObjectRef, shown: [bool; 2]) {
        let local = self.round.local_side as usize & 1;
        let o = self.objects.get_mut(r);
        o.set_visible(shown[local]);
        if shown[0] != shown[1] {
            o.sight = Sight::ByViewer(shown);
        }
    }

    /// `r` hidden from the viewers `hidden` names (by side), on top of what
    /// each sees of it now: where a console of the original clears
    /// `VISIBLE` by a rule of its own.
    pub fn hide_from(&mut self, r: ObjectRef, hidden: [bool; 2]) {
        let shown = [0u8, 1].map(|v| self.visible_to(r, v) && !hidden[v as usize]);
        self.set_visible_by_viewer(r, shown);
    }

    /// `r` hidden from a viewer blind to its side ([`Battle::sees`] on
    /// each console).
    pub fn hide_from_blind(&mut self, r: ObjectRef) {
        let alliance = self.objects.get(r).alliance;
        let hidden = [0u8, 1].map(|v| !self.sees(v, alliance));
        self.hide_from(r, hidden);
    }

    /// `r` shown on its own side's console only (`!is_remote` in the
    /// original: a Beast Out lock-on marker, an A-button charge glow).
    pub fn hide_from_other_side(&mut self, r: ObjectRef) {
        let alliance = self.objects.get(r).alliance & 1;
        self.hide_from(r, [0u8, 1].map(|v| v != alliance));
    }

    /// `to` seen by whoever sees `from`: an overlay or attachment that takes
    /// its owner's visibility.
    pub fn copy_visibility(&mut self, from: ObjectRef, to: ObjectRef) {
        let shown = [0u8, 1].map(|v| self.visible_to(from, v));
        self.set_visible_by_viewer(to, shown);
    }

    /// The other viewer's own custom screen's banner block while the custom
    /// screen is up (where its Program Advance's banner runs): the
    /// simulation's banner is the local side's screen's then, which only
    /// the local console shows.
    fn screen_banner(&self, viewer: u8) -> Option<&Banner> {
        let s = self.custom.sides[viewer as usize & 1].screen.as_ref()?;
        (viewer & 1 != self.round.local_side & 1 && self.round.mode == crate::battle::mode::CUSTOM).then_some(&s.hud)
    }

    /// The banner block as `viewer`'s console runs it (its step and timer:
    /// how far its banner has slid).
    pub fn banner_block_for(&self, viewer: u8) -> &Banner {
        self.screen_banner(viewer).unwrap_or(&self.banner)
    }

    /// The banner on screen as `viewer`'s console shows it.
    pub fn banner_for(&self, viewer: u8) -> Option<BannerId> {
        if let Some(block) = self.screen_banner(viewer) {
            return block.id.filter(|_| block.active);
        }
        let id = self.banner.id.filter(|_| self.banner.active)?;
        let local = self.round.local_side;
        if viewer == local {
            return Some(id);
        }
        let (telop, remote_telop) = (telop_banner(self, false), telop_banner(self, true));
        if let (Some(telop), Some(remote_telop)) = (telop, remote_telop) {
            if id == telop {
                return Some(remote_telop);
            }
            if id == remote_telop {
                return Some(telop);
            }
        }
        // (A result's banner is each console's own: its win's or its
        // loss's, as its own routine picks it.)
        if matches!(self.fight.state, fight::WIN | fight::LOSE) && id == self.result_banner(local, self.round.winner == local) {
            return Some(self.result_banner(viewer, self.round.winner == viewer));
        }
        Some(id)
    }

    /// The telop on `viewer`'s console's banner block: either player's
    /// where a telop is a banner; the viewer's own where telops run apart
    /// (the rules' `effects.telops`; the other player's is the second of
    /// [`Battle::telops_for`]).
    pub fn telop_for(&self, viewer: u8) -> Option<ShownTelop> {
        self.telops_for(viewer)[0].map(|(t, _)| t)
    }

    /// The telops on screen as `viewer`'s console shows them, each with the
    /// block it runs on (its step and timer: how far it has slid): the one
    /// on the banner block, and the one on the other player's block (none
    /// where a telop is a banner). Where telops run apart each console lays
    /// its own player's on the banner block: the other viewer's are the
    /// simulation's two the other way round (beside the simulation's
    /// banner, which that console might not have started).
    pub fn telops_for(&self, viewer: u8) -> [Option<(ShownTelop, Banner)>; 2] {
        self.telops_named(viewer, |_| viewer)
    }

    /// The telops on a screen of both consoles (a replay watched from both
    /// sides at once), each on its own player's side: laid as `left`'s
    /// console lays them ([`Battle::telops_for`]), each named as its user's
    /// console names it. A chip hidden from the other player alone is
    /// named; one hidden from both stays "????", as both consoles show it.
    pub fn telops_for_both(&self, left: u8) -> [Option<(ShownTelop, Banner)>; 2] {
        self.telops_named(left, |t| t.side)
    }

    /// The telops as `viewer`'s console lays them, each named as the
    /// console `namer` says names it.
    fn telops_named(&self, viewer: u8, namer: impl Fn(&Telop) -> u8) -> [Option<(ShownTelop, Banner)>; 2] {
        let on = |block: &Banner| {
            block.telop.filter(|_| block.active).map(|t| (ShownTelop { remote: t.side != viewer, ..shown(t, namer(&t)) }, *block))
        };
        let (banner, other) = (on(&self.banner), on(&self.remote_telop));
        match self.game_rules().effects.telops {
            Telops::Apart if viewer != self.round.local_side => [other, banner],
            _ => [banner, other],
        }
    }

    /// The round's result's banner on `viewer`'s console while it shows
    /// (its win's or its loss's, [`Battle::banner_for`]); none while
    /// another banner shows, or none.
    pub fn result_banner_for(&self, viewer: u8) -> Option<BannerId> {
        let id = self.banner_for(viewer)?;
        let decided = matches!(self.fight.state, fight::WIN | fight::LOSE);
        (decided && id == self.result_banner(viewer, self.round.winner == viewer)).then_some(id)
    }

    /// What `viewer`'s console shows of its navi's chips.
    pub fn chip_hud_for(&self, viewer: u8) -> crate::hud::ChipHud {
        self.chip_hud[viewer as usize & 1]
    }

    /// The chip the other player just used, while `viewer`'s console names
    /// it.
    pub fn used_chip_for(&self, viewer: u8) -> Option<crate::hud::UsedChip> {
        self.used_chips[(viewer ^ 1) as usize & 1]
    }

    /// How the round ended, for `viewer` (the result and the score are
    /// the local side's in `round_end`).
    pub fn round_end_for(&self, viewer: u8) -> Option<RoundEnd> {
        let end = self.round_end()?.clone();
        if viewer == self.round.local_side {
            return Some(end);
        }
        Some(match end {
            RoundEnd::Over(r) => RoundEnd::Over(r.for_other_side()),
            RoundEnd::NextRound { settings, score } => RoundEnd::NextRound { settings, score: score.for_other_side() },
            RoundEnd::Error(message) => RoundEnd::Error(message),
        })
    }

    /// The set's round wins so far, for `viewer`: theirs, then the other
    /// side's (the round's count is the local side's; a round's own is in it
    /// once the round is decided).
    pub fn wins_for(&self, viewer: u8) -> (u8, u8) {
        let r = &self.round;
        if viewer & 1 == r.local_side & 1 { (r.wins, r.losses) } else { (r.losses, r.wins) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::testing;

    fn battle() -> Battle {
        Battle::new(testing::round_setup(testing::LINK_BATTLE, testing::stats(100)), testing::content())
    }

    #[test]
    fn each_viewer_sees_its_own_name_and_result_banners() {
        let mut b = battle();
        let (telop, remote_telop) = (telop_banner(&b, false).unwrap(), telop_banner(&b, true).unwrap());
        b.start_banner(telop);
        assert_eq!((b.banner_for(0), b.banner_for(1)), (Some(telop), Some(remote_telop)));
        b.banner = Default::default();
        b.fight.state = fight::WIN;
        b.round.winner = 0;
        let navi = b.content.navi(b.content.navi_by_key(testing::MEGAMAN)).clone();
        b.start_banner(navi.win_banner);
        assert_eq!((b.banner_for(0), b.banner_for(1)), (Some(navi.win_banner), Some(navi.lose_banner)));
    }

    /// A Program Advance's banner runs on its player's own custom screen:
    /// the other console shows none of it, whichever side the simulation's
    /// local side is.
    #[test]
    fn a_program_advances_banner_shows_only_to_its_player() {
        use crate::battle::mode;
        use crate::content::BannerRole;
        let mut b = battle();
        let no_buttons = [crate::input::PlayerTick::default(); 2];
        for _ in 0..600 {
            if b.round.mode == mode::CUSTOM && b.round.init != 0 {
                break;
            }
            b.tick(&no_buttons, Default::default());
        }
        assert!(b.round.mode == mode::CUSTOM && b.custom.sides.iter().all(|s| s.screen.is_some()), "the custom screens open");
        let pa = b.roles().banner(BannerRole::ProgramAdvance);
        let steps = b.content.rules().effects.banner;
        for side in [0u8, 1] {
            let mut b = b.clone();
            assert!(b.custom.sides[side as usize].screen.as_mut().unwrap().hud.start(pa, true, steps));
            b.tick(&no_buttons, Default::default());
            assert_eq!(b.round.mode, mode::CUSTOM);
            assert_eq!((b.banner_for(side), b.banner_for(side ^ 1)), (Some(pa), None), "side {side}'s Program Advance");
            assert_eq!(b.banner_block_for(side).id, Some(pa));
        }
    }

    /// A navi that can be countered blinks blue on the other player's
    /// console alone, while that player is in Full Synchro, whichever side
    /// the simulation's local side is.
    #[test]
    fn the_counter_blink_shows_only_to_the_player_in_full_synchro() {
        use crate::battle::{TickEvents, mode};
        use crate::input::PlayerTick;
        use crate::kinds::player::{EmotionRole, hold_emotion};
        let mut b = Battle::new(crate::scenario::setup(), crate::scenario::content());
        let mut last = [0u16; 2];
        for _ in 0..2000 {
            if b.round.mode == mode::FIGHTING && b.fight.state == fight::FIGHTING {
                break;
            }
            let input = [0, 1].map(|side| {
                last[side] = crate::scenario::custom_buttons(&b, side, last[side]);
                PlayerTick::buttons(last[side])
            });
            b.tick(&input, TickEvents::default());
        }
        assert_eq!(b.fight.state, fight::FIGHTING, "the fight starts");
        let emotions = &b.game_rules().emotion;
        let synchro = emotions.holdable().into_iter().map(|(e, _)| e).find(|&e| emotions.role(e) == Some(EmotionRole::FullSynchro));
        let synchro = synchro.expect("the game has Full Synchro");
        for (watcher, blinker) in [(0u8, 1u8), (1, 0)] {
            let mut b = b.clone();
            let r = b.player(blinker).expect("the navi is in");
            let mut shown = Vec::new();
            for _ in 0..4 {
                assert!(hold_emotion(&mut b, watcher, synchro));
                let c = b.objects.get(r).collision.expect("the navi has its collision");
                b.collision.get_mut(c).counter_timer = 30;
                b.tick(&[PlayerTick::default(); 2], TickEvents::default());
                shown.push([0u8, 1].map(|v| b.look_for(r, v).color_shader));
            }
            assert!(shown.iter().any(|s| s[watcher as usize] == 0x7C00), "side {watcher} sees side {blinker}'s navi blink: {shown:x?}");
            assert!(shown.iter().all(|s| s[blinker as usize] != 0x7C00), "side {blinker} never sees its own navi blink: {shown:x?}");
        }
    }

    /// The round intro as each console draws it: its own navi shows at
    /// once and the other side's fades in, so the other side's console sees
    /// the local side's view with the navis the other way round.
    #[test]
    fn each_console_shows_its_own_navi_at_once_and_fades_the_other_in() {
        // (Either side the simulation's: the navis update in either order.)
        for local in [0usize, 1] {
            let mut setup = testing::round_setup(testing::LINK_BATTLE, testing::stats(100));
            setup.local_side = local as u8;
            let mut b = Battle::new(setup, testing::content());
            let no_buttons = [crate::input::PlayerTick::default(); 2];
            // By viewer, by navi: each tick's visibility, alpha and mosaic.
            let mut seen: [[Vec<(bool, Option<u8>, Option<u8>)>; 2]; 2] = Default::default();
            for _ in 0..200 {
                b.tick(&no_buttons, Default::default());
                let (Some(n0), Some(n1)) = (b.player(0), b.player(1)) else { continue };
                for v in 0..2u8 {
                    for (k, n) in [n0, n1].into_iter().enumerate() {
                        let look = b.look_for(n, v);
                        seen[v as usize][k].push((b.visible_to(n, v), look.alpha, look.mosaic));
                    }
                }
            }
            let other = local ^ 1;
            let fades = seen[local][other].iter().any(|&(_, alpha, _)| alpha.is_some_and(|a| a > 0 && a < 16));
            assert!(fades, "local side {local}: the other side's navi fades in");
            assert!(seen[local][local].iter().all(|&(_, alpha, _)| alpha.is_none()), "local side {local}: its own shows at once");
            assert_eq!(seen[other][local], seen[local][other], "local side {local}: the other console fades the local navi in alike");
            assert_eq!(seen[other][other], seen[local][local], "local side {local}: and shows its own at once");
        }
    }

    /// A mark each console turns to its own side faces each console's way,
    /// a palette set by console is each console's, and a copy takes both.
    #[test]
    fn a_look_set_by_console_is_each_consoles() {
        use crate::content::EffectRole;
        use nettai_content_api::{CoreApi, SpriteField, Value};
        let mut b = battle();
        let look = b.roles().effect(EffectRole::TrapMark);
        let e = crate::kinds::effect::spawn_by_console(&mut b, Default::default(), look, [0, 1], 0, 0).expect("a slot");
        b.run_objects();
        assert_eq!([0u8, 1].map(|v| b.look_for(e, v).hflip), [false, true]);
        b.sprite_set_by_console(e, SpriteField::Palette, [Value::Int(2), Value::Int(3)]).unwrap();
        assert_eq!([0u8, 1].map(|v| b.look_for(e, v).palette), [2, 3]);
        // A copy takes each console's (an attachment drawn as its owner).
        let worn = crate::kinds::effect::spawn(&mut b, Default::default(), look, 0, 0, 0).expect("a slot");
        b.sprite_copy(worn, e, &[SpriteField::Palette, SpriteField::HFlip]).unwrap();
        assert_eq!([0u8, 1].map(|v| (b.look_for(worn, v).palette, b.look_for(worn, v).hflip)), [(2, false), (3, true)]);
        // Set again, it is every console's.
        b.sprite_set(e, SpriteField::Palette, Value::Int(5)).unwrap();
        assert_eq!([0u8, 1].map(|v| b.look_for(e, v).palette), [5, 5]);
        assert!(b.sprite_set_by_console(e, SpriteField::VFlip, [Value::Bool(false), Value::Bool(true)]).is_err());
    }

    /// The result's banner as each game's routine picks it (`sub_80081A4`,
    /// `sub_800825A`; EXE5's 0x080074D2): the winner's navi's in the link
    /// battles the flow names, else "ENEMY DELETED", and the judge's
    /// ruling's "YOU WIN" and "YOU LOSE".
    #[test]
    fn a_results_banner_is_the_navis_or_the_roles() {
        use crate::battle::battle_flags;
        use crate::content::{BannerRole, NaviWinBanner};
        let content = |rule: NaviWinBanner| {
            let mut c: crate::content::Content = testing::build();
            c.define().unwrap_or_else(|e| panic!("{e}"));
            c.rules_mut().flow.navi_win_banner = rule;
            std::sync::Arc::new(c)
        };
        let with = |c: &std::sync::Arc<crate::content::Content>, link: bool| {
            let mut setup = testing::round_setup(testing::LINK_BATTLE, testing::megaman_on(c));
            crate::content::testing::on(&mut setup, &c);
            if !link {
                setup.settings.effects &= !crate::setup::effects::LINK;
            }
            Battle::new(setup, c.clone())
        };
        let exe6 = content(NaviWinBanner::LinkBattle);
        let mut b = with(&exe6, true);
        let navi = b.content.navi(b.content.navi_by_key(testing::MEGAMAN)).clone();
        let role = |b: &Battle, r: BannerRole| b.roles().banner(r);
        // A deletion: EXE6's link battle shows the navi's on both consoles.
        b.round.actor_count = [1, 0];
        assert_eq!((b.result_banner(0, true), b.result_banner(1, false)), (navi.win_banner, navi.lose_banner));
        // The judge's ruling: the navi's still for the winner, "YOU LOSE"
        // for the loser.
        b.round.actor_count = [1, 1];
        b.round.time_up = 1;
        assert_eq!((b.result_banner(0, true), b.result_banner(1, false)), (navi.win_banner, role(&b, BannerRole::LoseJudged)));
        // Outside a link battle a win is "ENEMY DELETED", or "YOU WIN".
        let mut b = with(&exe6, false);
        b.round.actor_count = [1, 0];
        assert_eq!(b.result_banner(0, true), role(&b, BannerRole::Win));
        b.round.actor_count = [1, 1];
        b.round.time_up = 1;
        assert_eq!(b.result_banner(0, true), role(&b, BannerRole::WinJudged));
        // EXE5's: the same in a link battle that is no operation battle,
        // on each console its own.
        let mut b = with(&content(NaviWinBanner::OperationBattle), true);
        b.round.actor_count = [1, 0];
        assert_eq!((b.result_banner(0, true), b.result_banner(1, false)), (role(&b, BannerRole::Win), navi.lose_banner));
        b.fight.state = fight::WIN;
        b.round.winner = 0;
        b.start_banner(b.result_banner(0, true));
        assert_eq!((b.banner_for(0), b.banner_for(1)), (Some(role(&b, BannerRole::Win)), Some(navi.lose_banner)));
        b.round.actor_count = [1, 1];
        b.round.time_up = 1;
        assert_eq!((b.result_banner(0, true), b.result_banner(1, false)), (role(&b, BannerRole::WinJudged), role(&b, BannerRole::LoseJudged)));
        // And the navi's in an operation battle, whatever the result.
        b.round.flags |= battle_flags::OWN_GAUGES;
        assert_eq!(b.result_banner(0, true), navi.win_banner);
        b.round.time_up = 0;
        b.round.actor_count = [1, 0];
        assert_eq!(b.result_banner(0, true), navi.win_banner);
        // EXE4's: "ENEMY DELETED" in every link battle, operation battle
        // or not.
        let mut b = with(&content(NaviWinBanner::Never), true);
        b.round.actor_count = [1, 0];
        b.round.flags |= battle_flags::OWN_GAUGES;
        assert_eq!((b.result_banner(0, true), b.result_banner(1, false)), (role(&b, BannerRole::Win), navi.lose_banner));
    }

    /// Each viewer sees the objects as its own console would: a blind
    /// player doesn't see the other side's, whoever the local side is, and
    /// a marker shows on its owner's console only. The `VISIBLE` flag stays
    /// the local side's.
    #[test]
    fn each_viewer_sees_what_its_console_would() {
        use crate::collision::f1;
        let mut b = battle();
        let no_buttons = [crate::input::PlayerTick::default(); 2];
        for _ in 0..4 {
            b.tick(&no_buttons, Default::default());
        }
        let navi = |b: &Battle, side: u8| b.player(side).expect("both navis are in");
        let (n0, n1) = (navi(&b, 0), navi(&b, 1));
        let blind = |b: &mut Battle, side: u8, on: bool| {
            let c = b.objects.get(navi(b, side)).collision.unwrap();
            let f = &mut b.collision.get_mut(c).f1;
            *f = if on { *f | f1::BLIND } else { *f & !f1::BLIND };
        };
        for r in [n0, n1] {
            b.objects.get_mut(r).set_visible(true);
        }
        // Side 1's player is blind: on its console, side 0's navi is gone.
        blind(&mut b, 1, true);
        assert!(!b.sees(1, 0) && b.sees(1, 1) && b.sees(0, 1));
        for r in [n0, n1] {
            b.hide_from_blind(r);
        }
        let seen = |b: &Battle| [n0, n1].map(|r| [0, 1].map(|viewer| b.visible_to(r, viewer)));
        assert_eq!(seen(&b), [[true, false], [true, true]]);
        // The flag is the local side's (side 0, who sees both).
        assert!(b.objects.get(n0).flags & flags::VISIBLE != 0);
        assert_eq!(b.flags_for(n0, 1) & flags::VISIBLE, 0);
        // Showing it again shows it to everyone.
        b.objects.get_mut(n0).set_visible(true);
        blind(&mut b, 1, false);
        blind(&mut b, 0, true);
        for r in [n0, n1] {
            b.hide_from_blind(r);
        }
        assert_eq!(seen(&b), [[true, true], [false, true]]);
        assert_eq!(b.objects.get(n1).flags & flags::VISIBLE, 0);
        // Something following side 1's navi is seen by whoever sees it.
        let follower = n0;
        b.copy_visibility(n1, follower);
        assert_eq!(seen(&b)[0], [false, true]);
        // A marker of side 1's shows on side 1's console only.
        b.objects.get_mut(follower).set_visible(true);
        b.objects.get_mut(follower).alliance = 1;
        b.hide_from_other_side(follower);
        assert_eq!(seen(&b)[0], [false, true]);
    }

    /// A screen of both consoles names a chip hidden from the other player
    /// (its user's console names it) and keeps "????" for one hidden from
    /// both, each telop on its own player's side.
    #[test]
    fn both_consoles_name_a_chip_its_user_sees() {
        let mut b = battle();
        let chip = crate::content::testing::chip_in(&b.content, crate::content::testing::SUN_GUN_3);
        b.start_banner(telop_banner(&b, true).unwrap());
        for (hidden, named) in [(TelopHidden::FromOpponent, TelopName::Chip(chip)), (TelopHidden::FromBoth, TelopName::Hidden)] {
            b.banner.telop = Some(Telop { side: 1, chip: Some(chip), damage: 0, doubled: false, bonus: 0, hidden });
            let [t, _] = b.telops_for_both(0);
            let (t, _) = t.unwrap();
            assert_eq!((t.name, t.remote), (named, true), "{hidden:?}");
            assert_eq!(b.telops_for(0)[0].unwrap().0.name, TelopName::Hidden);
        }
    }

    #[test]
    fn the_other_side_lost_what_the_local_side_won() {
        assert_eq!(BattleResult::Won.for_other_side(), BattleResult::Lost);
        let s = SetScore { wins: 2, losses: 1, round: 3, max_combo: 4 };
        assert_eq!(s.for_other_side(), SetScore { wins: 1, losses: 2, round: 3, max_combo: 4 });
        // The round wins so far, each viewer's first.
        let mut b = battle();
        (b.round.wins, b.round.losses) = (2, 1);
        let local = b.round.local_side;
        assert_eq!((b.wins_for(local), b.wins_for(local ^ 1)), ((2, 1), (1, 2)));
    }
}
