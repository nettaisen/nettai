--!strict
-- The glow over a navi (the original's actor object #0x5D, `sub_80C4828`;
-- docs/engine/dimming-chips.md §5.2): a copy of the side's navi, which
-- vanishes under it, flashing white. BugFix's controller spawns its first
-- variant; the other two are the original's, which nothing spawns.
--
--   bug_fix (`sub_80C493C`): white flashes further and further apart for
--     120 ticks, then 30 ticks, and the navi is back;
--   submerge (`sub_80C49E4`): white for 31 ticks, then the navi is back,
--     submerged for 480 ticks;
--   log_in (`sub_80C4A52`): white for 30 ticks with the log-in sound, then
--     as bug_fix's end.
--
-- When it is done it clears its spawner's `busy` flag. The behavior is
-- BN5's too (@exelib/bugfix/glow), made here with BN6's look: a pixel lower
-- down the field and higher than the navi, its parts stepping on.

local common = require("@exelib/bugfix/glow")

local glow = {}

-- Its variants (the original's first parameter, `off_80C4930`).
glow.BUG_FIX, glow.SUBMERGE, glow.LOG_IN = common.BUG_FIX, common.SUBMERGE, common.LOG_IN

local made = common.make {
    id = "bugfix/glow",
    sounds = {
        flash = asset.sound("bugfix-flash"),
        log_in = asset.sound("log-in"),
        submerge = asset.sound("invisible"),
    },
    offset = 1,
    bn5_parts = false,
}
glow.kind = made.kind
glow.spawn = made.spawn

return glow
