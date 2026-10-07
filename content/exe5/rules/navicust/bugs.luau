--!strict
-- EXE5's NaviCust bugs a hit inflicts (@exelib/navicust/bugs, EXE6's): 0x0801103E, after its light and dark part
-- (../light_dark: the codes 0xFD and 0xFC, and hit flag 0x400): the drains by their argument's flags, its own codes
-- (EXE6's but 0xF5 and 0xF8), and any other code's byte written at NaviStats + the code, past the side's block too
-- (0x08011142); then the NaviCust's hit bug (0x0801156E), whenever a hit asks a flinch or a drag. EXE5's intake has
-- none of EXE6's HP bug codes or its paralyzing, blinding one (its battle code tests none of 0xF4, 0xF6 and 0xF7).

local bugs = require("@exelib/navicust/bugs")
local statuses = require("../status")

local GAME: NaviCustBugTable = {
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
        [0xFB] = { kind = "strip" },
    },
    block = 0x60,
    past_block = "written",
    -- (Status bytes 0x32 and 0x22.)
    statuses = { hit_blind = statuses.blind_1200, hit_confuse = statuses.confuse_960 },
}

local navicust_bugs = {}

-- The rules' `navi_bug` (../light_dark's part first: ../init.luau).
navicust_bugs.navi_bug = bugs.navi_bug(GAME)

-- The rules' `navi_flinched`: the hit bug, with no gate of its own (0x0801156E).
function navicust_bugs.navi_flinched(side: number, navi: Object)
    bugs.hit_status_bug(GAME, side, navi)
end

return navicust_bugs
