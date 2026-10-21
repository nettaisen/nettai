//! A side's party: the navis a side brings and switches among, one on the
//! field at a time, each with a stats block of its own.
//!
//! The framework's navi switch (`kinds::player::actions::navi_switch`) has
//! always kept one block a side: the stats as the battle starts, which a
//! switch keeps the navi it leaves in and a switch knockout brings back
//! (the GBA's `eBattleNaviStats2034A60`, battle flag 0x40's link navi
//! switch). A party extends it to three: the side's own navi (member 0,
//! that same copy) and up to two partners its game's rules let join as the
//! round is set up (the DS's party battle, docs/design/exe5ds-map.md §4:
//! its member blocks at 0x021B9A5C + side·0x120 + member·0x60, filled by
//! 0x02097BDC). Who joins, what they bring and what a switch to them is
//! allowed are the rules'; the engine keeps the blocks, which one is on
//! the field, and the switches made, and runs the switch.
//!
//! A side whose rules add no partner has a party of one: everything here
//! is as the framework's single reserve was.

use crate::setup::NaviStats;

/// The most members a party holds: the side's own navi and two partners.
pub const MEMBERS: usize = 3;

/// A side's party (the DS's member blocks and party block, 0x021B85E8 +
/// side·0x24).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Party {
    /// The members' stats blocks, by member: 0 the side's own navi, its
    /// stats as the battle starts (after its rules set the round up); then
    /// the partners, as their rules made them. A block past `count` is
    /// empty. A member's block holds its stats while it is benched: a
    /// switch out writes the navi's into it, and a switch in takes it.
    pub members: [NaviStats; MEMBERS],
    /// How many members the party has (the party block's +0x15): 1, the
    /// side's own navi alone, to 3.
    pub count: u8,
    /// The member on the field (each member block's +0x58, its slot, which
    /// the side's stats carry while it fights).
    pub active: u8,
    /// The switches the side has made this round (the bottom screen's
    /// panel's +0xE, which a switch counts where it is accepted: the custom
    /// screen's state 0x40, 0x020639DE, or the in-battle switch's change,
    /// 0x0202C7EE). Its rules read it against their limit.
    pub switches: u8,
}

impl Party {
    /// A party of one: the side's own navi, `stats` its block.
    pub fn of(stats: NaviStats) -> Party {
        let mut members = [NaviStats::default(); MEMBERS];
        members[0] = stats;
        Party { members, count: 1, active: 0, switches: 0 }
    }

    /// Whether the side has partners to switch to.
    pub fn has_partners(&self) -> bool {
        self.count > 1
    }

    /// The members' blocks, in order.
    pub fn members(&self) -> &[NaviStats] {
        &self.members[..self.count as usize]
    }

    /// Member `m`'s block, if the party has it.
    pub fn member(&self, m: u8) -> Option<&NaviStats> {
        self.members().get(m as usize)
    }

    /// Member `m`'s block, to change, if the party has it.
    pub fn member_mut(&mut self, m: u8) -> Option<&mut NaviStats> {
        let n = self.count as usize;
        self.members[..n].get_mut(m as usize)
    }

    /// Add a member of these stats; its member number, or None when the
    /// party is full.
    pub fn join(&mut self, stats: NaviStats) -> Option<u8> {
        let m = self.count as usize;
        if m >= MEMBERS {
            return None;
        }
        self.members[m] = stats;
        self.count += 1;
        Some(m as u8)
    }

    /// The member whose block is of navi `navi` (the DS's 0x0200DEE8: the
    /// first, by member number), if one is.
    pub fn member_of(&self, navi: nettai_content_api::NaviHandle) -> Option<u8> {
        self.members().iter().position(|s| s.navi == navi).map(|m| m as u8)
    }
}
