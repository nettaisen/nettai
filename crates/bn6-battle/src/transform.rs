//! Cross and Beast Out transformations at the start of a turn: the
//! request each player's custom screen sends, and the sequencer that fades
//! the screen out, has the navis change form, and fades it back in
//! (`sub_801483C`). See docs/engine/battle-flow.md §3.4.1.

use crate::battle::Battle;
use crate::kinds::player;
use crate::setup::Form;

/// A player's transformation request for the coming turn (the game's
/// 0x10-byte transform record, sent with the chip exchange).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TransformRequest {
    /// The form to change into (Beast Out, a Cross, Beast Over...).
    pub form: Option<Form>,
    /// A Cross change (switching from one Cross to another).
    pub cross_change: Option<u8>,
}

impl TransformRequest {
    pub const NONE: TransformRequest = TransformRequest { form: None, cross_change: None };

    /// Decode a transform record: +0 the form, +4 the Cross change (0xFF
    /// = none for both). +1 and +3 are custom-screen bookkeeping nothing
    /// in battle reads, and +8 names the requesting navi object, which is
    /// always the side's player.
    pub fn from_bytes(b: &[u8]) -> TransformRequest {
        let opt = |v: u8| (v != 0xFF).then_some(v);
        TransformRequest { form: opt(b[0]).map(Form), cross_change: opt(b[4]) }
    }

    pub fn is_none(&self) -> bool {
        self.form.is_none() && self.cross_change.is_none()
    }
}

/// Where the sequencer is (`dword_20367F0`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransformPhase {
    /// `sub_80148EC`: fade the screen out.
    FadeOut,
    /// `sub_8014944`: tell the navis to change, wait until they have.
    Change,
    /// `sub_801498E`: fade the screen back in.
    FadeIn,
}

/// The transformation sequencer, run at the start of every turn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
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

/// Screen fade lengths the sequencer uses in netbattles.
const FADE_OUT_TICKS: u8 = 16;
const FADE_IN_TICKS: u8 = 17;

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
    /// side without a transformation checks whether its Beast Out ran out.
    fn sequencer_check(&mut self) {
        let mut transforming = false;
        for side in 0..2u8 {
            let req = self.transform_seq.requests[side as usize];
            let navi = self.player(side);
            if req.cross_change.is_some() {
                // sub_802DCDE
                panic!("Cross changes (sub_802DCDE) are not implemented yet");
            }
            if req.form.is_some() {
                transforming = true;
            } else if let Some(p) = navi {
                player::check_beast_out_end(self, p);
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
        if self.round.mode_copy == 1 {
            // Battle mode 1 fades with other screen fades (0x70 / 0x6C).
            panic!("transformations in battle mode 1 are not implemented yet");
        }
        let set = |b: &mut Battle, phase, started| b.transform_seq.state = SequencerState::Transform { phase, started };
        match phase {
            TransformPhase::FadeOut => {
                if !started {
                    // SetScreenFade(0x44, 0x10); the HUD hides.
                    self.fade.start();
                    self.fade.remaining = FADE_OUT_TICKS;
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
                    // SetScreenFade(0x40, 0x10).
                    self.fade.start();
                    self.fade.remaining = FADE_IN_TICKS;
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
