--!strict
-- LongSwrd: a slash on the two panels ahead.

local lockon = require("../rules/lockon")
local parts = require("../lib/swords/parts")
local regions = require("@exelib/regions")
local slash = require("../lib/swords/slash")
local slashes = require("../navis/megaman/forms/slashcross/slashes")

-- The slash over two panels.
local LONG_SLASH = new.effect { sprite = asset.sprite("sword-slash"), anim = 1, palette = 0 }

-- Its charged slash in SlashCross: over two panels.
local CHARGED = slashes.slash {
    wave = slashes.wave { region = regions.long, sprite = asset.sprite("sword-slash"), anim = 1 },
    blade = parts.blade,
    sound = parts.swing,
    lockon = lockon.firebrn1,
}

local chip: Chip = {
    codes = { "H", "L", "S", "*" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 2,
    mb = 25,
    damage = 100,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    beast = { lockon = lockon.firebrn1 },
    slot_in_limit = 3,
    action = slash.action {
        id = "longswrd/action",
        blade = parts.blade,
        hit = { region = regions.long, collision = parts.slashes_navis, hit_mod = 3 },
        effect = LONG_SLASH,
        sound = parts.swing,
        charged = CHARGED,
    },
}

return { longswrd = chip }
