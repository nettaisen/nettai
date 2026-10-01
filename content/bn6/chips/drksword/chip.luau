--!strict
-- DrkSword: a dark big slash on the two columns ahead (a dark chip: without bug frags, its user gets a Sword).

local parts = require("../../lib/swords/parts")
local regions = require("../../lib/regions")
local slash = require("../../lib/swords/slash")
local slashes = require("../../navis/00-megaman/forms/slashcross/slashes")
local lockon = require("../../rules/lockon")
local sword = require("../sword/chip")

return define.chip {
    id = "drksword",
    name = "DrkSword",
    description = "\n\n",
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 4,
    mb = 80,
    damage = 400,
    hit_param = 40,
    lockout = 0,
    flags = { "has_damage" },
    extra_flags = { 16, 32 },
    beast = { lockon = lockon.drksword },
    library = { number = 0, index = 66, sort = 0 },
    slot_in_limit = 1,
    dark_substitute = sword,
    hp_bug = 2,
    action = slash.action {
        id = "drksword/action",
        blade = parts.blades.sword,
        hit = { region = regions.wide_long, collision = parts.slashes_navis, hit_mod = 3 },
        effect = parts.effects.dark,
        sound = parts.sounds.big_swing,
        charged = slashes.dark,
    },
}
