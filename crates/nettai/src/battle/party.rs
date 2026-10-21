//! A party battle's party beside the battle (EXE5 DS's bottom screen, its party panel: docs/design/exe5ds-map.md §4.2):
//! the player's switches asked of the battle, and what the window shows of the party.
//!
//! **The switches** are two controls of the app's own ([`crate::input::SWITCH_LEFT`], [`crate::input::SWITCH_RIGHT`]:
//! the keys and pad buttons bound to them, and on a phone the pad's two switch buttons), never buttons of the battle:
//! a press asks the player to switch to the member the panel's button names (`Player::ask_switch`, which the next
//! tick carries once, as the DS's X and Y request a frame), the left Y's and the right X's (`nettai_frontend::party::
//! Panel`). A button that names no member does nothing; what the battle refuses (the round's limit, a member at 0 HP)
//! is its own, with its sound.
//!
//! **What is shown** is read from the battle each frame through the frontend (`nettai_frontend::party::view`: the
//! members, their HP, the member on the field, the switches made), with the panel's two buttons and the round's limit,
//! which the build's rules say (`Overview::switches`, asked once of the side's setup as the battle first shows a
//! party). Nothing of it is the simulation's: the battle's picture shows only what the original's top screen draws.

use crate::input::{SWITCH_LEFT, SWITCH_RIGHT, SWITCHES};
use nettai_frontend::party::{self, Panel};
use nettai_ui::{Hint, PartyHud, PartyMember};
use slint::{ModelRc, VecModel};

/// What the window keeps of a battle's party.
#[derive(Default)]
pub struct PartyState {
    panel: Panel,
    /// The switches held last frame: a press is one held now and not then.
    held: u16,
    /// The round's limit of switches, once asked of the side's setup (none inside: its rules have none).
    limit: Option<Option<u32>>,
    /// What the window was told last.
    shown: Option<PartyHud>,
}

/// The switches' limit side `side` of `b` plays by, as its rules' overview says it of the side's setup.
fn limit_of(b: &nettai_battle::Battle, game: &str, side: u8) -> Option<u32> {
    let block = b.setup.players[side as usize & 1].rules.clone()?;
    let side = nettai_match::Side { facts: nettai_match::facts::Facts::of_block(block) };
    nettai_match::build::Checked::of(&b.content, game, &side).overview(&b.content, &side)?.switches
}

impl PartyState {
    /// The frame's buttons held, `held` (the app's, the switches among them): the switch a press asks for put to the
    /// player, and the battle's buttons alone given back.
    pub fn take_switches(&mut self, player: &mut nettai_frontend::Player, side: u8, held: u16) -> u16 {
        let switches = held & SWITCHES;
        let pressed = switches & !self.held;
        self.held = switches;
        self.panel.follow(party::view(player.battle(), side).as_ref());
        let button = if pressed & SWITCH_LEFT != 0 {
            Some(0)
        } else if pressed & SWITCH_RIGHT != 0 {
            Some(1)
        } else {
            None
        };
        if let Some(member) = button.and_then(|b| self.panel.member(b)) {
            player.ask_switch(member);
        }
        held & !SWITCHES
    }

    /// What the window shows of side `side`'s party this frame (none: it has no partners), told to the UI where it
    /// changed. `faces` gives a navi's face; `keys` the two switches' controls as bound.
    pub fn show(&mut self, shell: &nettai_ui::Shell, b: &nettai_battle::Battle, game: &str, side: u8, faces: impl Fn(nettai_content_api::NaviHandle) -> slint::Image, keys: [Hint; 2]) {
        let view = party::view(b, side);
        self.panel.follow(view.as_ref());
        let hud = match &view {
            None => PartyHud::default(),
            Some(v) => {
                let limit = *self.limit.get_or_insert_with(|| limit_of(b, game, side));
                let place = |m: Option<u8>| m.map_or(-1, |m| m as i32);
                let members: Vec<PartyMember> = v.members.iter().map(|m| PartyMember { face: faces(m.navi), hp: m.hp as i32, max_hp: m.max_hp as i32 }).collect();
                let [left_key, right_key] = keys;
                PartyHud {
                    shown: true,
                    members: ModelRc::new(VecModel::from(members)),
                    active: self.panel.active() as i32,
                    left: place(self.panel.member(0)),
                    right: place(self.panel.member(1)),
                    switches: v.switches as i32,
                    limit: limit.map_or(v.switches as i32, |l| l as i32),
                    left_key,
                    right_key,
                }
            }
        };
        if self.shown.as_ref().is_some_and(|s| same(s, &hud)) {
            return;
        }
        shell.set_party(hud.clone());
        self.shown = Some(hud);
    }
}

/// Whether two of what the window shows are alike (an image's own pixels aren't compared: a navi's face is its).
fn same(a: &PartyHud, b: &PartyHud) -> bool {
    use slint::Model;
    let members = |h: &PartyHud| h.members.iter().map(|m| (m.hp, m.max_hp)).collect::<Vec<_>>();
    (a.shown, a.active, a.left, a.right, a.switches, a.limit, &a.left_key, &a.right_key) == (b.shown, b.active, b.left, b.right, b.switches, b.limit, &b.left_key, &b.right_key) && members(a) == members(b)
}
