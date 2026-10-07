--!strict
-- EXE5's bugs a hit inflicts (@exelib/navicust/bugs, EXE6's): 0x0801103E's NaviCust bugs (a code below 0x64 names a
-- NaviStats byte; EXE5's own stats' by their names), where a drain's argument goes by its flags, and its special
-- codes, which are EXE6's but 0xF5 and 0xF8 (no chip gives them: they write their bytes as any other code, not
-- ported); the codes 0xFD and 0xFC are its light and dark part's (../light_dark), before these. The HP bug's codes,
-- the paralyzing, blinding one, the HP bug's marker and the NaviCust's hit bug are EXE6's.

local bugs = require("@exelib/navicust/bugs")
local statuses = require("../status")

return bugs.hooks({
    -- The rules' `stats` by their bytes: the sun (+0x22), what the NaviCust does to the drops and the encounters
    -- (+0x26, +0x28), the soul's extra turns (+0x32, signed) and Hub Style (+0x4C).
    own = {
        [0x22] = { name = "sun", as = "flag" },
        [0x26] = { name = "chip_drops" },
        [0x28] = { name = "encounters" },
        [0x32] = { name = "soul_turn_bonus", as = "signed" },
        [0x4C] = { name = "hub_style" },
    },
    drain_by_flags = true,
    specials = {
        [0xFF] = { kind = "blanks" },
        [0xFE] = bugs.panel_trail(4, 4),
        [0xFA] = bugs.panel_trail(4, 2),
        [0xF9] = bugs.panel_trail(4, 1),
        [0xF5] = { kind = "unported" },
        [0xFB] = { kind = "strip" },
        [0xF8] = { kind = "unported" },
    },
    -- (The hit bug's: status bytes 0x32 and 0x22.)
    statuses = {
        paralyze = statuses.paralyze_150,
        blind = statuses.blind_1200,
        hit_blind = statuses.blind_1200,
        hit_confuse = statuses.confuse_960,
    },
})
