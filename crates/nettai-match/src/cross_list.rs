//! A side's Cross list: the forms its form list offers in place of its
//! version's own (EXE6's Cross window: up to five Crosses, of either
//! version, in this order). nettai's extension: the original's window
//! offers its version's five (docs/engine/custom-screen.md §4.1). A round's
//! setup carries it as the `cross_list` fact (`facts::write`).

use nettai_content_api::FormHandle;

/// The most forms a list holds: the engine's form list's entries.
pub const CROSSES: usize = nettai_battle::custom::screen::CROSSES;

/// The forms a side names for its form list, in the order the list offers
/// them (those not used this round, and not the navi's starting form).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct CrossList {
    forms: [Option<FormHandle>; CROSSES],
}

impl CrossList {
    /// The list of `forms`, at most [`CROSSES`].
    pub fn new(forms: &[FormHandle]) -> CrossList {
        assert!(forms.len() <= CROSSES, "a Cross list holds at most {CROSSES} forms, not {}", forms.len());
        let mut list = CrossList::default();
        for (slot, &f) in list.forms.iter_mut().zip(forms) {
            *slot = Some(f);
        }
        list
    }

    /// The form in place `place`.
    pub fn get(&self, place: u8) -> Option<FormHandle> {
        self.forms.get(place as usize).copied().flatten()
    }

    /// The forms, in order.
    pub fn forms(&self) -> impl Iterator<Item = FormHandle> + '_ {
        self.forms.iter().flatten().copied()
    }
}
