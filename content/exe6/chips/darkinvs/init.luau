--!strict
-- DarkInvs, a dark chip: its navi runs the Cross special, fighting on its
-- own with chips by its HP (its change, `sub_80E9784`, on the navi-changing
-- chips' controller: lib/navi_boost/controller).

local dimming_chips = require("@exelib/dimming")
local invisibl = require("./invisibl").invisibl
local navi_boost = require("../lib/navi_boost/controller")

-- `sub_80E9784`: the navi's cross-special request (EXE6's rules/beast
-- starts its Cross special: rules/beast/cross-special.luau).
local CROSS_SPECIAL = navi_boost.change(function(me: Object)
    navi_boost.user_of(me):set_request("takeover", true)
    dimming_chips.done(me)
end)

local chip: Chip = {
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 4,
    mb = 64,
    damage = 0,
    hit_param = 50,
    lockout = 0,
    flags = { "dimming" },
    extra_flags = { 16, 32 },
    slot_in_limit = 1,
    dark_substitute = invisibl,
    dimming = navi_boost.hook(CROSS_SPECIAL),
}

return { darkinvs = chip }
