--!strict
-- HawkCut1 to HawkCut3: the dimming chips' variant 0x39, EXE4's own: a controller (effect object #0x68, 0x080E95E0:
-- run as the navi chips', lib/navi_chips' long course, 0x080E9628) on which the user warps out (30 ticks), ProtoMan
-- comes and slashes the panels ahead twice (./protoman), 30 ticks pass and the user warps back in (30 ticks); each chip
-- its own controller (each chip's own comment above its definition).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local navi_chips = require("../lib/navi_chips")
local protoman = require("@self/protoman")

local function controller(id: string): (user: Object, spec: DimmingChipSpec) -> Object?
    return navi_chips.controller({ id = id, name = "HawkCut", spawn = protoman.spawn, steps = navi_chips.LONG }).hook
end

-- HawkCut1 (EXE4 chip 0x0A9).
local hawkcut1: Chip = {
    codes = { "H", "N", "O" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 1,
    mb = 40,
    damage = 80,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    dimming = controller("hawkcut1/controller"),
}

-- HawkCut2 (EXE4 chip 0x0AA).
local hawkcut2: Chip = {
    codes = { "F", "S", "V" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 2,
    mb = 50,
    damage = 90,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    dimming = controller("hawkcut2/controller"),
}

-- HawkCut3 (EXE4 chip 0x0AB).
local hawkcut3: Chip = {
    codes = { "I", "N", "Z" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 3,
    mb = 60,
    damage = 100,
    hit_param = 148,
    lockout = 0,
    flags = { "dimming", "has_damage", "library" },
    dimming = controller("hawkcut3/controller"),
}

return { hawkcut1 = hawkcut1, hawkcut2 = hawkcut2, hawkcut3 = hawkcut3 }
