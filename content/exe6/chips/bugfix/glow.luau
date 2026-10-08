--!strict
-- The glow over a navi (the original's actor object #0x5D, `sub_80C4828`;
-- docs/engine/dimming-chips.md §5.2): a copy of the side's navi, which
-- vanishes under it, flashing white. BugFix's controller spawns it with its
-- flashes (`sub_80C493C`, the first row: white flashes further and further
-- apart for 120 ticks), then 30 ticks, and the navi is back. When it is
-- done it clears its spawner's `busy` flag. The behavior is EXE5's too
-- (@exelib/bugfix/glow), made here with EXE6's look: a pixel lower down the
-- field and higher than the navi, its parts stepping on.

local common = require("@exelib/bugfix/glow")

local glow = {}

local made = common.make {
    id = "bugfix/glow",
    offset = 1,
    exe5_parts = false,
}
glow.kind = made.kind
glow.spawn = made.spawn

return glow
