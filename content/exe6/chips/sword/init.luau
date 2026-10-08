--!strict
-- Sword: a slash on the panel ahead.

local lockon = require("../rules/lockon")
local parts = require("../lib/swords/parts")
local regions = require("@exelib/regions")
local slash = require("../lib/swords/slash")
local slashes = require("../navis/megaman/forms/slashcross/slashes")

-- Sword's slash over the panel ahead.
local SWORD_SLASH = new.effect { sprite = asset.sprite("sword-slash"), anim = 2, palette = 0 }

-- Its charged slash in SlashCross: the plain sword's (the original's row 0): over the panel ahead.
local CHARGED = slashes.slash {
    wave = slashes.wave { region = regions.single, sprite = asset.sprite("sword-slash"), anim = 2 },
    blade = parts.blade,
    sound = parts.swing,
    lockon = lockon.thunder,
}

local chip: Chip = {
    codes = { "H", "L", "S", "*" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 0,
    mb = 8,
    damage = 80,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    beast = { lockon = lockon.thunder },
    slot_in_limit = 3,
    action = slash.action {
        id = "sword/action",
        blade = parts.blade,
        hit = { region = regions.single, collision = parts.slashes_navis, hit_mod = 3 },
        effect = SWORD_SLASH,
        sound = parts.swing,
        charged = CHARGED,
    },
}

return { sword = chip }
