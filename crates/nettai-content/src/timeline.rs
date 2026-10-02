//! M4A tracks as timelines: the commands a track executes, in order, each
//! at its tick, with the control flow (GOTO, PATT/PEND, REPT, FINE) played
//! out and the loop it settles into marked.
//!
//! A timeline is what a MIDI file can say (events at times, one loop), and
//! it is exact: [`linearize`] follows the driver's control flow step by
//! step, so a track [`encode`]d from a timeline plays the same commands at
//! the same ticks as the original. Two tracks with equal timelines are
//! indistinguishable to the driver.
//!
//! Optional note arguments are made explicit (a note without a key plays
//! the track's last key, and so on), so every note names its key and
//! velocity and every end-of-tie its key.

use m4a::bank::{Command, Track};
use std::collections::HashMap;
use std::fmt;

/// One command of a timeline: anything but waits and control flow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Event {
    pub tick: u32,
    pub command: Command,
}

/// How a timeline ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ending {
    /// The track ends (FINE) at `tick`.
    Fine { tick: u32 },
    /// The events from index `start` on repeat forever: the loop begins at
    /// `start_tick` and each pass lasts `end_tick - start_tick` ticks.
    /// Events at `end_tick` belong to the pass that ends there.
    Loop { start: usize, start_tick: u32, end_tick: u32 },
}

/// A track's commands laid out in time.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Timeline {
    pub events: Vec<Event>,
    pub ending: Ending,
}

impl Timeline {
    /// The tick of the end: FINE, or the end of the loop's first pass.
    pub fn end_tick(&self) -> u32 {
        match self.ending {
            Ending::Fine { tick } => tick,
            Ending::Loop { end_tick, .. } => end_tick,
        }
    }

    /// The loop's (start, end) ticks, if it loops.
    pub fn loop_ticks(&self) -> Option<(u32, u32)> {
        match self.ending {
            Ending::Loop { start_tick, end_tick, .. } => Some((start_tick, end_tick)),
            Ending::Fine { .. } => None,
        }
    }

    /// For a looping timeline: the earliest tick from which it repeats
    /// whole ticks at a time, and its period. From that tick on, every
    /// tick's events come back `period` ticks later, so a loop may start at
    /// any tick at or after it ([`Timeline::with_loop`]) and be marked by
    /// tick alone, as MIDI loop markers are.
    pub fn loop_from(&self) -> Option<(u32, u32)> {
        let Ending::Loop { start, start_tick, end_tick } = self.ending else { return None };
        let period = end_tick - start_tick;
        let e = &self.events;
        let k = e.len() - start;
        if k == 0 {
            return Some((e.last().map_or(0, |x| x.tick + 1), period));
        }
        // Stretch the repeating part back over intro events that already
        // repeat.
        let mut s = start;
        while s > 0 && e[s - 1].command == e[s - 1 + k].command && e[s - 1].tick + period == e[s - 1 + k].tick {
            s -= 1;
        }
        let after_intro = if s > 0 { e[s - 1].tick + 1 } else { 0 };
        // A pass can't end on the tick the next one starts on.
        let after_seam = (e[s + k - 1].tick + 1).saturating_sub(period);
        Some((after_intro.max(after_seam), period))
    }

    /// The same timeline with its loop starting at tick `start` and lasting
    /// `period` ticks: `start` at or after [`Timeline::loop_from`]'s tick,
    /// `period` a multiple of the loop's own. The result has no intro
    /// events at `start` or later and no loop events at its end tick.
    pub fn with_loop(&self, start: u32, period: u32) -> Timeline {
        let Ending::Loop { start: body, start_tick, end_tick } = self.ending else { return self.clone() };
        let own = end_tick - start_tick;
        debug_assert!(period.is_multiple_of(own) && self.loop_from().is_some_and(|(from, _)| start >= from));
        let end = start + period;
        let mut events: Vec<Event> = Vec::new();
        let mut first_in_loop = None;
        let mut push = |e: Event, events: &mut Vec<Event>| {
            if e.tick >= start && first_in_loop.is_none() {
                first_in_loop = Some(events.len());
            }
            events.push(e);
        };
        for &e in &self.events[..body] {
            push(e, &mut events);
        }
        if body < self.events.len() {
            'passes: for pass in 0.. {
                for &e in &self.events[body..] {
                    let e = Event { tick: e.tick + pass * own, ..e };
                    if e.tick >= end {
                        break 'passes;
                    }
                    push(e, &mut events);
                }
            }
        }
        let first = first_in_loop.unwrap_or(events.len());
        Timeline { events, ending: Ending::Loop { start: first, start_tick: start, end_tick: end } }
    }
}

/// A track that can't be laid out.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TimelineError {
    /// It loops without time passing (the driver would hang).
    ZeroLengthLoop { at: usize },
    /// It runs longer than any song reasonably does before settling.
    TooLong,
    /// A MEMACC jumps on what the memory area holds: what the track plays
    /// depends on the game, not on time alone.
    Conditional { at: usize },
}

impl fmt::Display for TimelineError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            TimelineError::ZeroLengthLoop { at } => write!(f, "loops at command {at} without time passing"),
            TimelineError::TooLong => write!(f, "doesn't settle into a loop or end within {MAX_EVENTS} events"),
            TimelineError::Conditional { at } => {
                write!(f, "jumps at command {at} on the memory area (a conditional MEMACC), which a timeline can't say")
            }
        }
    }
}

impl std::error::Error for TimelineError {}

/// Pattern calls nest this deep (the driver ends a track that goes deeper).
const MAX_CALLS: usize = 3;
const MAX_EVENTS: usize = 1 << 20;

/// Everything that decides what a track does next.
#[derive(Clone, PartialEq, Eq, Hash)]
struct FlowState {
    pc: usize,
    calls: Vec<usize>,
    repeats: u8,
    key: u8,
    velocity: u8,
}

/// Lay a track out as the driver plays it from its start.
pub fn linearize(track: &Track) -> Result<Timeline, TimelineError> {
    let cmds = &track.commands;
    let mut s = FlowState { pc: 0, calls: Vec::new(), repeats: 0, key: 0, velocity: 0 };
    let mut tick = 0u32;
    let mut events = Vec::new();
    // Where each state was first seen: (events so far, tick).
    let mut seen: HashMap<FlowState, (usize, u32)> = HashMap::new();
    loop {
        if let Some(&(start, start_tick)) = seen.get(&s) {
            if start_tick == tick {
                return Err(TimelineError::ZeroLengthLoop { at: s.pc });
            }
            return Ok(Timeline { events, ending: Ending::Loop { start, start_tick, end_tick: tick } });
        }
        seen.insert(s.clone(), (events.len(), tick));
        if events.len() > MAX_EVENTS {
            return Err(TimelineError::TooLong);
        }
        // Running off the end plays as FINE.
        let Some(&cmd) = cmds.get(s.pc) else { return Ok(Timeline { events, ending: Ending::Fine { tick } }) };
        s.pc += 1;
        match cmd {
            Command::Wait(n) => tick += n as u32,
            Command::Fine => return Ok(Timeline { events, ending: Ending::Fine { tick } }),
            Command::Goto(i) => s.pc = i as usize,
            Command::Call(i) => {
                if s.calls.len() >= MAX_CALLS {
                    return Ok(Timeline { events, ending: Ending::Fine { tick } });
                }
                s.calls.push(s.pc);
                s.pc = i as usize;
            }
            Command::Return => {
                if let Some(r) = s.calls.pop() {
                    s.pc = r;
                }
            }
            Command::Repeat { count, target } => {
                if count == 0 {
                    s.pc = target as usize;
                } else {
                    s.repeats = s.repeats.wrapping_add(1);
                    if s.repeats < count {
                        s.pc = target as usize;
                    } else {
                        s.repeats = 0;
                    }
                }
            }
            Command::Note { gate, key, velocity } => {
                s.key = key.unwrap_or(s.key);
                s.velocity = velocity.unwrap_or(s.velocity);
                let command = Command::Note { gate, key: Some(s.key), velocity: Some(s.velocity) };
                events.push(Event { tick, command });
            }
            Command::EndTie { key } => {
                s.key = key.unwrap_or(s.key);
                events.push(Event { tick, command: Command::EndTie { key: Some(s.key) } });
            }
            Command::MemAcc { op: m4a::bank::MemOp::JumpIf { .. }, .. } => {
                return Err(TimelineError::Conditional { at: s.pc - 1 });
            }
            command => events.push(Event { tick, command }),
        }
    }
}

/// Waits are written in steps of at most this (the longest wait M4A data
/// has a command for).
const MAX_WAIT: u32 = 96;

/// A straight-line track that plays a timeline: its events with waits
/// between them, then FINE or a GOTO back to the loop's start.
pub fn encode(t: &Timeline) -> Track {
    let mut commands = Vec::new();
    let mut now = 0u32;
    let wait_to = |commands: &mut Vec<Command>, now: &mut u32, tick: u32| {
        while *now < tick {
            let n = (tick - *now).min(MAX_WAIT);
            commands.push(Command::Wait(n as u8));
            *now += n;
        }
    };
    let mut loop_index = None;
    for (i, e) in t.events.iter().enumerate() {
        if let Ending::Loop { start, start_tick, .. } = t.ending
            && i == start
        {
            wait_to(&mut commands, &mut now, start_tick);
            loop_index = Some(commands.len());
        }
        wait_to(&mut commands, &mut now, e.tick);
        commands.push(e.command);
    }
    match t.ending {
        Ending::Fine { tick } => {
            wait_to(&mut commands, &mut now, tick);
            commands.push(Command::Fine);
        }
        Ending::Loop { start_tick, end_tick, .. } => {
            let target = match loop_index {
                Some(i) => i,
                None => {
                    wait_to(&mut commands, &mut now, start_tick);
                    commands.len()
                }
            };
            wait_to(&mut commands, &mut now, end_tick);
            commands.push(Command::Goto(target as u32));
        }
    }
    Track { commands }
}

#[cfg(test)]
mod tests {
    use super::*;
    use Command::*;

    fn note(gate: u8, key: Option<u8>, velocity: Option<u8>) -> Command {
        Note { gate, key, velocity }
    }

    #[test]
    fn patterns_and_repeats_unroll() {
        // 0: VOL, 1: PATT 5, 2: REPT 2 -> 1, 3: FINE; 5: N12, W12, PEND.
        let t = Track {
            commands: vec![
                Volume(100),
                Call(5),
                Repeat { count: 2, target: 1 },
                Fine,
                Fine,
                note(12, Some(60), Some(100)),
                Wait(12),
                Return,
            ],
        };
        let tl = linearize(&t).unwrap();
        let ticks: Vec<u32> = tl.events.iter().map(|e| e.tick).collect();
        assert_eq!(ticks, vec![0, 0, 12]);
        assert_eq!(tl.ending, Ending::Fine { tick: 24 });
        assert_eq!(linearize(&encode(&tl)).unwrap(), tl);
    }

    #[test]
    fn loops_are_found_with_their_key_state() {
        // The loop's first pass plays key 50 (from the intro) for the bare
        // note; later passes play 62, so the loop settles one pass later.
        let t = Track {
            commands: vec![
                note(6, Some(50), Some(90)),
                Wait(6),
                note(6, None, None), // 2: loop
                Wait(6),
                note(6, Some(62), None),
                Wait(6),
                Goto(2),
            ],
        };
        let tl = linearize(&t).unwrap();
        // The state first repeats after the explicit key 62 (tick 12).
        assert_eq!(tl.ending, Ending::Loop { start: 3, start_tick: 12, end_tick: 24 });
        assert_eq!(tl.events[1].command, note(6, Some(50), Some(90)));
        assert_eq!(tl.events[3].command, note(6, Some(62), Some(90)));
        let back = encode(&tl);
        assert_eq!(linearize(&back).unwrap(), tl);
        // Whole ticks repeat from just after the last key-50 note (tick 6).
        assert_eq!(tl.loop_from(), Some((7, 12)));
        let aligned = tl.with_loop(24, 24);
        assert_eq!(aligned.ending, Ending::Loop { start: 4, start_tick: 24, end_tick: 48 });
        assert!(aligned.events[..4].iter().all(|e| e.tick < 24));
        assert_eq!(aligned.events.iter().map(|e| e.tick).collect::<Vec<_>>(), vec![0, 6, 12, 18, 24, 30, 36, 42]);
        assert_eq!(linearize(&encode(&aligned)).unwrap().with_loop(24, 24), aligned);
    }

    #[test]
    fn loops_that_start_with_a_wait_keep_their_start() {
        let t = Track { commands: vec![Volume(1), Wait(4), Pan(3), Wait(8), Goto(1)] };
        let tl = linearize(&t).unwrap();
        assert_eq!(tl.ending, Ending::Loop { start: 1, start_tick: 0, end_tick: 12 });
        assert_eq!(linearize(&encode(&tl)).unwrap(), tl);
        // A track that only idles.
        let t = Track { commands: vec![Volume(1), Wait(96), Goto(1)] };
        let tl = linearize(&t).unwrap();
        assert_eq!(tl.ending, Ending::Loop { start: 1, start_tick: 0, end_tick: 96 });
        assert_eq!(linearize(&encode(&tl)).unwrap(), tl);
    }

    #[test]
    fn hanging_loops_and_deep_calls() {
        assert_eq!(linearize(&Track { commands: vec![Volume(1), Goto(0)] }), Err(TimelineError::ZeroLengthLoop { at: 0 }));
        // Calls nested past the driver's limit end the track.
        let t = Track { commands: vec![Wait(1), Call(0)] };
        assert_eq!(linearize(&t).unwrap().ending, Ending::Fine { tick: 4 });
    }
}
