//! Cross and Beast Out transformations at the start of a turn: the
//! request each player's custom screen sends, and the sequencer that fades
//! the screen out, has the navis change form, and fades it back in
//! (`sub_801483C`). See docs/engine/battle-flow.md §3.4.1.

use crate::battle::{Battle, FadeMode};
use crate::kinds::player;
use nettai_content_api::{FormHandle, NaviHandle};

/// A player's transformation request for the coming turn (the game's
/// 0x10-byte transform record, sent with the chip exchange).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TransformRequest {
    /// The form to change into (Beast Out, a Cross, Beast Over...).
    pub form: Option<FormHandle>,
    /// A Cross change: the navi to change to.
    pub cross_change: Option<NaviHandle>,
    /// BN5's Soul Unison (`sub_8015952`'s record, 0x0203C940): the soul's
    /// turns (+3) and whether it is Chaos Unison (+1).
    pub turns: u8,
    pub chaos: bool,
}

impl TransformRequest {
    pub const NONE: TransformRequest = TransformRequest { form: None, cross_change: None, turns: 0, chaos: false };

    pub fn is_none(&self) -> bool {
        self.form.is_none() && self.cross_change.is_none()
    }
}

/// Where the sequencer is (`dword_20367F0`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SequencerState {
    /// Check each side (`sub_801486C`): Beast Out running out, Cross
    /// changes, and whether anyone transforms.
    #[default]
    Check,
    /// Someone transforms (`sub_80148CC`).
    Transform { phase: TransformPhase, started: bool },
    /// Wait for form reversions and Cross changes to finish
    /// (`sub_8014A00`).
    Wait,
}

/// The phases of a transformation, each with an entry tick.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TransformPhase {
    /// `sub_80148EC`: fade the screen out.
    FadeOut,
    /// `sub_8014944`: tell the navis to change, wait until they have.
    Change,
    /// `sub_801498E`: fade the screen back in.
    FadeIn,
}

/// The transformation sequencer, run at the start of every turn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct TransformSequencer {
    pub state: SequencerState,
    pub busy: bool,
    /// Its working copy of both sides' requests; requests are dropped as
    /// they are carried out.
    pub requests: [TransformRequest; 2],
}

impl TransformSequencer {
    /// `sub_801482C`: start over (the requests are kept).
    pub fn restart(&mut self) {
        self.state = SequencerState::Check;
        self.busy = true;
    }
}

/// The reversion a mid-battle custom-screen request waits for before the
/// sequencer runs (`dword_203C970`, `sub_802D6A0` / `sub_802D6C4`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct CustomReversion {
    /// +3: the navis were checked.
    pub checked: bool,
    /// +4: still waiting.
    pub busy: bool,
    /// +8 / +0xC: both sides' navis.
    pub navis: [Option<crate::object::ObjectRef>; 2],
}

impl Battle {
    /// `sub_802D6A0`: note both navis and start waiting.
    pub(crate) fn start_custom_reversion(&mut self) {
        self.custom_reversion = CustomReversion { checked: false, busy: true, navis: [self.player(0), self.player(1)] };
    }

    /// `sub_802D6C4`: one step; true while waiting. The first step would
    /// knock the navis out of their Crosses, but its test
    /// (`sub_802DD1E`) is always false; then it waits while either navi
    /// is being knocked out.
    pub(crate) fn step_custom_reversion(&mut self) -> bool {
        let rev = &mut self.custom_reversion;
        if !rev.checked {
            rev.checked = true;
            return rev.busy;
        }
        let navis = rev.navis;
        let knocked_out = |b: &Battle, n: Option<crate::object::ObjectRef>| {
            n.is_some_and(|p| {
                b.objects.get(p).actor.is_some_and(|a| b.actors.get(a).status & crate::actor::status::CROSS_KNOCKOUT != 0)
            })
        };
        if !knocked_out(self, navis[0]) && !knocked_out(self, navis[1]) {
            self.custom_reversion.busy = false;
        }
        self.custom_reversion.busy
    }
}

/// The sequencer's screen fades step by 0x10: 16 ticks out, 17 back in.
const FADE_SPEED: u8 = 0x10;

impl Battle {
    /// `sub_80147E4`: start the sequencer with this turn's requests. The
    /// navis read their side's request from `turn_transforms`.
    pub(crate) fn start_transform_sequencer(&mut self) {
        let requests = self.transform_requests;
        self.turn_transforms = requests;
        self.transform_seq = TransformSequencer { requests, ..Default::default() };
        self.transform_seq.restart();
    }

    /// `sub_801483C`: one step of the sequencer; true while it is busy.
    pub(crate) fn step_transform_sequencer(&mut self) -> bool {
        match self.transform_seq.state {
            SequencerState::Check => self.sequencer_check(),
            SequencerState::Transform { phase, started } => self.sequencer_transform(phase, started),
            SequencerState::Wait => self.sequencer_wait(),
        }
        self.transform_seq.busy
    }

    /// `sub_801486C`: a side asking for a Cross change gets it started; a
    /// side without a transformation has its rules check whether its form's
    /// time ran out (BN6's beast system: Beast Out, `sub_80159C6`).
    fn sequencer_check(&mut self) {
        let mut transforming = false;
        for side in 0..2u8 {
            let req = self.transform_seq.requests[side as usize];
            let navi = self.player(side);
            if req.cross_change.is_some() {
                // A Cross change is asked for, and the check runs too (the
                // form isn't looked at).
                if let Some(p) = navi {
                    player::actions::cross_change::request_change(self, p);
                    self.notify_side(side, nettai_content_api::SystemHook::TurnCheck);
                }
            } else if req.form.is_some() {
                transforming = true;
            } else if navi.is_some() {
                self.notify_side(side, nettai_content_api::SystemHook::TurnCheck);
            }
        }
        self.transform_seq.state = if transforming {
            SequencerState::Transform { phase: TransformPhase::FadeOut, started: false }
        } else {
            SequencerState::Wait
        };
    }

    /// `sub_80148CC`.
    fn sequencer_transform(&mut self, phase: TransformPhase, started: bool) {
        // Battle mode 1 fades in other colors (0x70 / 0x6C), and shows
        // other HUD parts; the timing is the same.
        let mode1 = self.round.mode_copy == 1;
        let set = |b: &mut Battle, phase, started| b.transform_seq.state = SequencerState::Transform { phase, started };
        match phase {
            TransformPhase::FadeOut => {
                if !started {
                    // The HUD hides.
                    let mode = if mode1 { FadeMode::Mode1TransformOut } else { FadeMode::TransformOut };
                    self.fade.start(mode, FADE_SPEED);
                    set(self, phase, true);
                }
                if !self.fade.active() {
                    set(self, TransformPhase::Change, false);
                }
            }
            TransformPhase::Change => {
                if !started {
                    // sub_801596E
                    for side in 0..2u8 {
                        if self.transform_seq.requests[side as usize].form.is_some()
                            && let Some(p) = self.player(side)
                        {
                            player::request_form_change(self, p);
                        }
                    }
                    set(self, phase, true);
                    return;
                }
                // sub_801597C
                let changing = (0..2u8).filter_map(|side| self.player(side)).any(|p| player::changing_form(self, p));
                if changing {
                    return;
                }
                for req in &mut self.transform_seq.requests {
                    req.form = None;
                }
                set(self, TransformPhase::FadeIn, false);
            }
            TransformPhase::FadeIn => {
                if !started {
                    let mode = if mode1 { FadeMode::Mode1TransformIn } else { FadeMode::TransformIn };
                    self.fade.start(mode, FADE_SPEED);
                    set(self, phase, true);
                }
                if !self.fade.active() {
                    // The HUD comes back.
                    self.transform_seq.state = SequencerState::Wait;
                }
            }
        }
    }

    /// `sub_8014A00`: done once no navi is reverting its form or changing
    /// Cross.
    fn sequencer_wait(&mut self) {
        let navis = [self.player(0), self.player(1)];
        if navis.iter().flatten().any(|&p| player::reverting_form(self, p)) {
            return;
        }
        for (side, navi) in navis.iter().enumerate() {
            if navi.is_some_and(|p| player::changing_cross(self, p)) {
                return;
            }
            self.transform_seq.requests[side].cross_change = None;
        }
        self.transform_seq.busy = false;
    }
}
