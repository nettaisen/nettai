//! Direct connect is native only: a browser can't listen on a port, nor dial
//! one without a description from the other side. So a browser's link never
//! meets directly: [`Direct`] has no value, and nothing here runs.

use crate::peer::PeerConnection;
use crate::{Config, Error};

pub(crate) enum Direct {}

impl Direct {
    pub(crate) fn port(&self) -> Option<u16> {
        match *self {}
    }

    pub(crate) fn describe(&self) -> String {
        match *self {}
    }

    pub(crate) fn hosts(&self) -> bool {
        match *self {}
    }

    pub(crate) fn answer(&mut self, _config: &Config, _generation: u32, _outage: bool) -> Result<Option<(PeerConnection, u32)>, Error> {
        match *self {}
    }

    pub(crate) fn dial(&self, _config: &Config, _generation: u32) -> Option<Result<PeerConnection, Error>> {
        match *self {}
    }
}
