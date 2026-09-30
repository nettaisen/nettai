--!strict
-- LifeSrd: a big slash on the two columns ahead; the Program Advance of Sword, WideSwrd, LongSwrd (or
-- WideBlde, LongBlde).

local parts = require("../../lib/swords/parts")
local regions = require("../../lib/regions")
local slash = require("../../lib/swords/slash")
local sword = require("../sword/chip")
local wideswrd = require("../wideswrd/chip")
local longswrd = require("../longswrd/chip")
local wideblde = require("../wideblde/chip")
local longblde = require("../longblde/chip")

return define.chip {
    id = "lifesrd",
    name = "LifeSrd",
    codes = { "A", "B", "C", "*" },
    element = "null", family = "program_advance", class = "program_advance",
    rarity = 0, mb = 5, damage = 400, hit_param = 40, lockout = 0,
    flags = { "has_damage", "library" },
    library = { number = 339, index = 20, sort = 0 },
    slot_in_limit = 1,
    program_advances = {
        { order = 0, sequence = { sword, wideswrd, longswrd } },
        { order = 1, sequence = { sword, wideblde, longblde } },
    },
    action = slash.action {
        blade = parts.blades.sword,
        hit = { region = regions.wide_long, collision = parts.slashes_navis, hit_mod = 3 },
        effect = parts.effects.big,
        sound = parts.sounds.big_swing,
    },
}
