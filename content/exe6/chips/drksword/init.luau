--!strict
-- DrkSword: a dark big slash on the two columns ahead (a dark chip: without bug frags, its user gets a Sword).

local lockon = require("../rules/lockon")
local parts = require("../lib/swords/parts")
local regions = require("@exelib/regions")
local slash = require("../lib/swords/slash")
local slashes = require("../navis/megaman/forms/slashcross/slashes")
local sword = require("./sword").sword

-- The dark big slash.
local DARK_SLASH = new.effect { sprite = asset.sprite("big-slash"), anim = 0, palette = 2 }

-- Its charged slash in SlashCross: the big slash over two columns, in palette 10.
local CHARGED = slashes.slash {
    wave = slashes.wave { region = regions.wide_long, sprite = asset.sprite("big-slash"), anim = 0, palette = 10 },
    blade = parts.blade,
    sound = parts.big_swing,
    lockon = lockon.drksword,
}

local chip: Chip = {
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
    slot_in_limit = 1,
    dark_substitute = sword,
    hp_bug = 2,
    action = slash.action {
        id = "drksword/action",
        blade = parts.blade,
        hit = { region = regions.wide_long, collision = parts.slashes_navis, hit_mod = 3 },
        effect = DARK_SLASH,
        sound = parts.big_swing,
        charged = CHARGED,
    },
}

return { drksword = chip }
