//! A side's party as a host shows it beside the battle: what EXE5 DS draws on its bottom screen for the party battle
//! (docs/design/exe5ds-map.md §4.2, the party panel), read from the battle each frame. Nothing here is kept in the
//! battle, and nothing of it reaches the simulation but the switch the player asks for (`Player::ask_switch`), which
//! is input.
//!
//! **The members** ([`view`]): each with its navi and its HP: the navi on the field's its object's, live; a benched
//! member's its block's (the party's member blocks keep a member's stats while it is benched). The member on the field
//! is the custom screen's while the player has switched there (the screen's navi, which the turn's start fields), else
//! the party's.
//!
//! **The panel's two buttons** ([`Panel`]): the DS's party panel has a left and a right button, each naming the member
//! a press switches to (its +5 and +6), Y the left and X the right (the panel's state 0x02083254), a touch on either
//! the same. A party starts with member 1 on the left and member 2 on the right (none where it has one partner); a
//! switch puts the member that leaves on the button that named the one that came (0x02036EEC), so which member a
//! button names follows the round's switches. The panel follows the member on the field from frame to frame: a change
//! from `a` to `b` puts `a` on the button that named `b` (whatever made the change: a switch in battle, one on the
//! custom screen, the knockout's take-back), and a new round (the switches counted from none again) starts it over.
//! A change undone (a rollback's wrong prediction shown, then taken back) is undone on the buttons too: the change is
//! its own inverse.

use nettai_battle::Battle;
use nettai_content_api::NaviHandle;

/// A member of a party, as the panel shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Member {
    pub navi: NaviHandle,
    pub hp: u16,
    pub max_hp: u16,
}

/// A side's party, as the panel shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartyView {
    /// The members, in order: the side's own navi first.
    pub members: Vec<Member>,
    /// The member on the field (the custom screen's, where the player switched there).
    pub active: u8,
    /// The switches the side made this round.
    pub switches: u8,
}

/// Side `side`'s party as the battle stands, where it has partners (none: a side of one, which has no panel).
pub fn view(b: &Battle, side: u8) -> Option<PartyView> {
    let party = &b.party[side as usize & 1];
    if !party.has_partners() {
        return None;
    }
    let screen = b.custom.sides[side as usize & 1].screen.as_ref().and_then(|s| s.member);
    let active = screen.unwrap_or(party.active);
    let live = b.player(side).map(|r| b.objects.get(r).hp);
    let members = party
        .members()
        .iter()
        .enumerate()
        .map(|(m, s)| {
            // (The navi on the field: its object's HP and the side's stats; a benched member: its block. On the custom
            // screen the screen's member isn't fielded yet: its block too.)
            if m as u8 == party.active && screen.is_none() {
                let stats = &b.stats[side as usize & 1];
                Member { navi: s.navi, hp: live.unwrap_or(stats.hp), max_hp: stats.max_hp }
            } else {
                Member { navi: s.navi, hp: s.hp, max_hp: s.max_hp }
            }
        })
        .collect();
    Some(PartyView { members, active, switches: party.switches })
}

/// The party panel's two buttons, left and right, as they follow a side's party.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Panel {
    /// The member each button names, left and right.
    buttons: [Option<u8>; 2],
    /// The party it followed last: its members, the member on the field, the switches made.
    count: u8,
    active: u8,
    switches: u8,
}

impl Panel {
    /// The buttons a party of `count` members starts with: member 1 on the left, member 2 on the right.
    fn fresh(count: u8) -> [Option<u8>; 2] {
        [(count > 1).then_some(1), (count > 2).then_some(2)]
    }

    /// Follow the party as this frame shows it (none: the side has no partners now).
    pub fn follow(&mut self, party: Option<&PartyView>) {
        let Some(p) = party else {
            *self = Panel::default();
            return;
        };
        let count = p.members.len() as u8;
        // (A new party, or a new round: the switches counted from none again. A party that has made no switch is at
        // its start, whatever was shown before: a switch shown and taken back by a rollback too.)
        if count != self.count || (p.switches == 0 && self.switches != 0) {
            *self = Panel { buttons: Panel::fresh(count), count, active: 0, switches: 0 };
        }
        if p.active != self.active {
            for b in &mut self.buttons {
                if *b == Some(p.active) {
                    *b = Some(self.active);
                }
            }
            self.active = p.active;
        }
        self.switches = p.switches;
    }

    /// The member button `button` (0 the left, Y; 1 the right, X) names, if any.
    pub fn member(&self, button: usize) -> Option<u8> {
        self.buttons.get(button).copied().flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn party(count: usize, active: u8, switches: u8) -> PartyView {
        let member = Member { navi: NaviHandle(0), hp: 1, max_hp: 1 };
        PartyView { members: vec![member; count], active, switches }
    }

    /// The DS's panel: member 1 on the left (Y) and 2 on the right (X); a switch puts the member leaving on the
    /// button that named the one coming; the take-back to member 0 the same; a new round starts it over; one partner
    /// has the left button alone.
    #[test]
    fn the_panel_follows_the_switches() {
        let mut panel = Panel::default();
        let at = |p: &Panel| [p.member(0), p.member(1)];
        panel.follow(Some(&party(3, 0, 0)));
        assert_eq!(at(&panel), [Some(1), Some(2)]);
        // Y to 1: the left button names MegaMan.
        panel.follow(Some(&party(3, 1, 1)));
        assert_eq!(at(&panel), [Some(0), Some(2)]);
        // X to 2: the right button names 1.
        panel.follow(Some(&party(3, 2, 2)));
        assert_eq!(at(&panel), [Some(0), Some(1)]);
        // 2 deleted, MegaMan taken back: the left button names 2.
        panel.follow(Some(&party(3, 0, 2)));
        assert_eq!(at(&panel), [Some(2), Some(1)]);
        // A change shown and taken back (a rollback) leaves it as it was.
        panel.follow(Some(&party(3, 1, 3)));
        panel.follow(Some(&party(3, 0, 2)));
        assert_eq!(at(&panel), [Some(2), Some(1)]);
        // The next round.
        panel.follow(Some(&party(3, 0, 0)));
        assert_eq!(at(&panel), [Some(1), Some(2)]);
        // One partner.
        let mut one = Panel::default();
        one.follow(Some(&party(2, 0, 0)));
        assert_eq!(at(&one), [Some(1), None]);
        one.follow(Some(&party(2, 1, 1)));
        assert_eq!(at(&one), [Some(0), None]);
        one.follow(None);
        assert_eq!(at(&one), [None, None]);
    }
}
