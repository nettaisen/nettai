//! The link panel's figures ([`super::Battle`]'s, in an online match): what the connection's corner opens (the
//! user: "you should allow clicking the ping indicator in pvp to bring up a tango-style link quality panel
//! thing"; docs/design/app-ui.md, "In an online match").
//!
//! The library's `NetStatus` has every figure the panel shows; what it doesn't keep is the past, and the panel
//! graphs the round trip's last ten seconds, as Tango's telemetry graphs its ping. So the round trip is sampled
//! ten times a second for the whole match, whether the panel is open or not (it has a graph to show the moment it
//! opens), and while it is open the panel's figures are made again at each sample: ten times a second, often
//! enough to read as live, not so often that the digits blur.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use nettai_frontend::driver::NetStatus;
use nettai_ui::LinkFigures;

/// How often the round trip is sampled, and how many samples the graph shows: ten seconds of them.
const EVERY: Duration = Duration::from_millis(100);
const KEPT: usize = 100;

/// The least round trip at the graph's top, in ms, and the step its top goes up by when a sample is above it.
const GRAPH_TOP: u64 = 100;
const GRAPH_STEP: u64 = 50;

/// The round trip's recent past.
#[derive(Default)]
pub struct LinkHistory {
    /// The newest last; none while the link was down, or before a round trip was measured (a gap).
    samples: VecDeque<Option<u64>>,
    /// When the last sample was taken.
    last: Option<Instant>,
}

impl LinkHistory {
    /// A history of these samples, oldest first (the screenshot pass's fixed figures): the last ten seconds' of
    /// them.
    #[cfg_attr(not(feature = "tour"), allow(dead_code))]
    pub fn of(samples: impl IntoIterator<Item = Option<u64>>) -> LinkHistory {
        let mut samples: VecDeque<_> = samples.into_iter().collect();
        while samples.len() > KEPT {
            samples.pop_front();
        }
        LinkHistory { samples, last: None }
    }

    /// Take a sample of the round trip if one is due. True when one was: the panel's figures are made again
    /// then.
    pub fn sample(&mut self, now: Instant, net: &NetStatus) -> bool {
        if self.last.is_some_and(|last| now.saturating_duration_since(last) < EVERY) {
            return false;
        }
        self.last = Some(now);
        if self.samples.len() == KEPT {
            self.samples.pop_front();
        }
        self.samples.push_back(if net.reconnecting.is_some() { None } else { net.rtt_ms });
        true
    }

    /// The panel's figures: `net`'s, and the graph of the samples.
    pub fn figures(&self, net: &NetStatus) -> LinkFigures {
        let int = |n: u64| i32::try_from(n).unwrap_or(i32::MAX);
        let kb = |bytes: u64| int(bytes.div_ceil(1024));
        let (graph, graph_area, top) = self.graph();
        LinkFigures {
            jitter: net.jitter_ms.map_or(-1, |ms| ms.round() as i32),
            loss: (net.loss * 1000.0).round() as i32,
            input_delay: int(net.input_delay.into()),
            rollbacks: int(net.rollbacks),
            last_rollback: int(net.last_rollback.into()),
            deepest_rollback: int(net.max_rollback.into()),
            skew: net.skew,
            lead: int(net.lead.into()),
            waits: int(net.waits),
            sent: int(net.sent.datagrams),
            sent_kb: kb(net.sent.bytes),
            received: int(net.received.datagrams),
            received_kb: kb(net.received.bytes),
            reconnects: int(net.reconnects.into()),
            graph: graph.into(),
            graph_area: graph_area.into(),
            graph_top: int(top),
        }
    }

    /// The graph, as paths in a box 99 wide and 100 tall (`LinkFigures`): its line, the area under it, and the
    /// round trip at its top (the least step over every sample in view, and 100 ms at the least). The newest
    /// sample is at the right edge and each older one a step to the left, so the graph comes in from the right
    /// until it is full; a run of samples between gaps is a line and an area of its own.
    fn graph(&self) -> (String, String, u64) {
        let top = self.samples.iter().flatten().max().map_or(GRAPH_TOP, |&max| max.div_ceil(GRAPH_STEP) * GRAPH_STEP).max(GRAPH_TOP);
        let skip = KEPT - self.samples.len();
        let point = |i: usize, ms: u64| ((skip + i) as f64, 100.0 - ms.min(top) as f64 * 100.0 / top as f64);
        let (mut line, mut area) = (String::new(), String::new());
        let mut run: Vec<(f64, f64)> = Vec::new();
        let mut close = |run: &mut Vec<(f64, f64)>| {
            if let (Some(&(x0, _)), Some(&(x1, _))) = (run.first(), run.last()) {
                for (n, (x, y)) in run.iter().enumerate() {
                    line.push_str(&format!("{} {x} {y:.1} ", if n == 0 { 'M' } else { 'L' }));
                }
                area.push_str(&format!("M {x0} 100 "));
                for (x, y) in run.iter() {
                    area.push_str(&format!("L {x} {y:.1} "));
                }
                area.push_str(&format!("L {x1} 100 Z "));
            }
            run.clear();
        };
        for (i, sample) in self.samples.iter().enumerate() {
            match sample {
                Some(ms) => run.push(point(i, *ms)),
                None => close(&mut run),
            }
        }
        close(&mut run);
        (line.trim_end().to_string(), area.trim_end().to_string(), top)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nettai_frontend::connection::Traffic;

    fn with_rtt(rtt_ms: Option<u64>) -> NetStatus {
        NetStatus { rtt_ms, ..NetStatus::default() }
    }

    /// A sample a tenth of a second at the most, ten seconds of them kept; the graph's newest at the right edge,
    /// its top a step over the highest, a gap where the link was down.
    #[test]
    fn samples_make_the_graph() {
        let mut history = LinkHistory::default();
        let start = Instant::now();
        assert!(history.sample(start, &with_rtt(Some(40))));
        assert!(!history.sample(start + Duration::from_millis(50), &with_rtt(Some(40))));
        assert!(history.sample(start + EVERY, &with_rtt(None)));
        assert!(history.sample(start + 2 * EVERY, &with_rtt(Some(130))));
        let (line, area, top) = history.graph();
        assert_eq!(top, 150);
        assert_eq!(line, "M 97 73.3 M 99 13.3");
        assert_eq!(area, "M 97 100 L 97 73.3 L 97 100 Z M 99 100 L 99 13.3 L 99 100 Z");
        // Fifteen seconds of a steady 20 ms: ten are kept, the graph across the whole box, its top the least.
        for n in 3..153 {
            history.sample(start + n * EVERY, &with_rtt(Some(20)));
        }
        let (line, _, top) = history.graph();
        assert_eq!((history.samples.len(), top), (KEPT, GRAPH_TOP));
        assert!(line.starts_with("M 0 80.0 L 1 80.0 ") && line.ends_with("L 99 80.0"), "{line}");
    }

    /// The figures as the panel shows them: the loss in tenths of a percent, the bytes in KB rounded up, no
    /// jitter as -1.
    #[test]
    fn figures_are_the_panels_units() {
        let net = NetStatus { loss: 0.0123, sent: Traffic { datagrams: 600, bytes: 1025 }, skew: -2, ..NetStatus::default() };
        let figures = LinkHistory::default().figures(&net);
        assert_eq!((figures.loss, figures.sent, figures.sent_kb, figures.jitter, figures.skew, figures.graph_top), (12, 600, 2, -1, -2, 100));
    }
}
