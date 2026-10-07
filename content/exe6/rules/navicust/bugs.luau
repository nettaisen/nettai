--!strict
-- EXE6's bugs a hit inflicts (@exelib/navicust/bugs, which EXE5's share): `sub_80139F6`'s NaviCust bugs (a code
-- below 0x64 names a NaviStats byte; EXE6's own stats' by their names), its special codes, the HP bug's codes and the
-- paralyzing, blinding one (`sub_801A6B4`, `sub_801A720`), the HP bug's marker and the NaviCust's hit bug.

local bugs = require("@exelib/navicust/bugs")
local sparks = require("../../lib/sparks")
local statuses = require("../status")

return bugs.hooks({
    -- The rules' `stats` by their bytes: the Beast Out counter (+0x21), the sun (+0x22), what the NaviCust does to
    -- the drops and the encounters (+0x26, +0x28), ChpShufl's and NumbrOpn's flags (+0x60; +0x61, which the custom
    -- screen tests for 1).
    own = {
        [0x21] = { name = "beast_out_counter" },
        [0x22] = { name = "sun", as = "flag" },
        [0x26] = { name = "chip_drops" },
        [0x28] = { name = "encounters" },
        [0x60] = { name = "chip_shuffle", as = "flag" },
        [0x61] = { name = "number_open", as = "one" },
    },
    unheld = { [0x20] = "the version (a side's fact, rules/api's `exe6.version`)" },
    drain_by_flags = false,
    specials = {
        [0xFF] = { kind = "blanks" },
        [0xFE] = bugs.panel_trail(4, 4),
        [0xFA] = bugs.panel_trail(4, 2),
        [0xF9] = bugs.panel_trail(4, 1),
        [0xF5] = bugs.panel_trail(3, 1),
        [0xFB] = { kind = "strip" },
        -- (Effect 0xE's spark, sound 0x8E.)
        [0xF8] = { kind = "uninstall", spark = sparks.uninstall, sound = asset.sound("fade") },
    },
    statuses = {
        paralyze = statuses.paralyze_150,
        blind = statuses.blind_1200,
        hit_blind = statuses.blind_1200,
        hit_confuse = statuses.confuse_960,
    },
})
