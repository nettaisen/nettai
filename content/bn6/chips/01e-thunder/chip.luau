--!strict
-- Registration by number (docs/design/content-model-v2.md §12,
-- "Transitional"): a record that names the original's action 0x1F runs
-- this module, which runs the chip's own shot (chips/thunder,
-- chips/darkthnd). Thunder is a definition; the records that still reach
-- it are DarkThnd's (a dark chip) and Thunder's, which the ruleset gives
-- for a dark chip without a bug frag (chip_use.rs `SUBSTITUTES`). The two
-- differ only in their ball's panels (the record's second parameter: 12 is
-- DarkThnd's). It goes when dark chips name their substitutes (phase C,
-- step 10).

local darkthnd = require("../darkthnd/chip")
local shoot = require("../thunder/shoot")
local thunder = require("../thunder/chip")

local by_number = {
    state = shoot.STATE,
}

-- DarkThnd's ball's panels.
local DARK_PANELS = 0x0C

function by_number.update(me: Object, s: any)
    local a = if me:attack_param(2) == DARK_PANELS then darkthnd.action else thunder.action
    a.update(me, s)
end

return by_number
