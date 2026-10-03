--!strict
-- LifeSrd: a big slash on the two columns ahead; the Program Advance of Sword, WideSwrd, LongSwrd (or
-- WideBlde, LongBlde).

local parts = require("../../lib/swords/parts")
local regions = require("../../lib/regions")
local slash = require("../../lib/swords/slash")
local slashes = require("../../navis/megaman/forms/slashcross/slashes")
local lockon = require("../../rules/lockon")
local longblde = require("../longblde/chip")
local longswrd = require("../longswrd/chip")
local sword = require("../sword/chip")
local wideblde = require("../wideblde/chip")
local wideswrd = require("../wideswrd/chip")

return define.chip {
    id = "bn6:lifesrd",
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 400,
    hit_param = 40,
    lockout = 0,
    flags = { "has_damage", "library" },
    beast = { lockon = lockon.drksword },
    library = { number = 339, index = 20, sort = 0 },
    slot_in_limit = 1,
    program_advances = {
        { order = 0, sequence = { sword, wideswrd, longswrd } },
        { order = 1, sequence = { sword, wideblde, longblde } },
    },
    action = slash.action {
        id = "bn6:lifesrd/action",
        blade = parts.blades.sword,
        hit = { region = regions.wide_long, collision = parts.slashes_navis, hit_mod = 3 },
        effect = parts.effects.big,
        sound = parts.sounds.big_swing,
        charged = slashes.life,
    },
}
