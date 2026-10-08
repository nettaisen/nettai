--!strict
-- BugFix's dimming controller (the original's effect object #0x3B,
-- `sub_80E4910`; docs/engine/dimming-chips.md §5.1): the usual dimming
-- phases (@exelib/dimming), with an effect (`sub_80E4954`) that puts the glow
-- over the user's side's navi (glow.luau), fixes the user's side's
-- NaviCust bugs, spends the Beast Out counter again unless the navi has one,
-- and waits for the glow. The behavior is EXE5's too
-- (@exelib/bugfix/controller), made here with EXE6's glow and Beast Out.

local common = require("@exelib/bugfix/controller")
local glow = require("./glow")

local bugfix = {}

local made = common.make {
    id = "bugfix/controller",
    glow = glow.spawn,
    flash = asset.sound("bugfix-flash"),
    beast_out = true,
}
bugfix.kind = made.kind

-- `sub_80E49A2`: the chip's `dimming` hook.
bugfix.spawn = made.spawn

return bugfix
