//! A player's SP navi deletion times (the rules' fact the engine knows as
//! `PlayerFact::SpTimes`, the save's `byte_203EB00`: [`crate::Facts::sp_times`])
//! as people read and write them: a time each, by the SP navi chip whose
//! damage goes by it, as `mm:ss.cc`. The game keeps frames and shows them as the BCD time
//! `sub_8000D84` makes (hundredths of the frames past the second, rounded
//! down); a written time is the fewest frames that show as at least it, so
//! each time the game shows reads back as itself.

/// The time `frames` shows as, `mm:ss.cc`.
pub fn format(frames: u16) -> String {
    let f = frames as u32;
    let (minutes, rest) = (f / 3600, f % 3600);
    let (seconds, frames) = (rest / 60, rest % 60);
    format!("{minutes:02}:{seconds:02}.{:02}", frames * 100 / 60)
}

/// The frames a time (`mm:ss.cc`, `m:ss.cc` or `ss.cc`) is: the fewest
/// that show as it or more.
pub fn parse(text: &str) -> Result<u16, String> {
    let bad = || format!("{text:?} is no time (mm:ss.cc)");
    let t = text.trim();
    let (minutes, rest) = match t.split_once(':') {
        Some((m, rest)) => (m.parse::<u32>().map_err(|_| bad())?, rest),
        None => (0, t),
    };
    let (seconds, hundredths) = match rest.split_once('.') {
        Some((s, c)) if c.len() == 2 => (s.parse::<u32>().map_err(|_| bad())?, c.parse::<u32>().map_err(|_| bad())?),
        Some(_) => return Err(bad()),
        None => (rest.parse::<u32>().map_err(|_| bad())?, 0),
    };
    if seconds >= 60 && t.contains(':') {
        return Err(format!("{text:?}: a minute has 60 seconds"));
    }
    // (The frames into the second that show these hundredths: the fewest
    // whose `frames * 100 / 60` is at least them.)
    let frames = minutes * 3600 + seconds * 60 + (hundredths * 60).div_ceil(100);
    u16::try_from(frames).map_err(|_| format!("{text:?}: a deletion time is at most {}", format(u16::MAX)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_time_the_game_shows_reads_back_as_itself() {
        for frames in 0..=u16::MAX {
            assert_eq!(parse(&format(frames)).map(|f| format(f)), Ok(format(frames)), "{frames} frames");
        }
        assert_eq!(format(0), "00:00.00");
        assert_eq!(format(600), "00:10.00");
        assert_eq!(format(601), "00:10.01");
        assert_eq!(format(3600 + 59 * 60 + 59), "01:59.98");
    }

    #[test]
    fn a_time_between_frames_is_the_next_one_up() {
        // 12.02 s falls between 12.01 (721 frames) and 12.03 (722).
        assert_eq!(parse("0:12.02"), Ok(722));
        assert_eq!(parse("12.00"), Ok(720));
        assert_eq!(parse("12"), Ok(720));
        // Past a step's whole second, the next step (`sub_8010AE4`).
        assert!(parse("00:12.01").unwrap() > 720);
        assert!(parse("1:60.00").is_err());
        assert!(parse("99:00.00").is_err());
        assert!(parse("0:12.5").is_err());
    }
}
