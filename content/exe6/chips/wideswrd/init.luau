--!strict
-- WideSwrd: a slash on the column ahead.

local lockon = require("../rules/lockon")
local parts = require("../lib/swords/parts")
local regions = require("@exelib/regions")
local slash = require("../lib/swords/slash")
local slashes = require("../navis/megaman/forms/slashcross/slashes")

-- The slash over a column.
local WIDE_SLASH = new.effect { sprite = asset.sprite("sword-slash"), anim = 0, palette = 0 }

-- Its charged slash in SlashCross: over a column.
local CHARGED = slashes.slash {
    wave = slashes.wave { region = regions.wide, sprite = asset.sprite("sword-slash"), anim = 0 },
    blade = parts.blade,
    sound = parts.swing,
    lockon = lockon.widesht,
}

local chip: Chip = {
    codes = { "H", "L", "S", "*" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 1,
    mb = 12,
    damage = 80,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    beast = { lockon = lockon.widesht },
    slot_in_limit = 3,
    action = slash.action {
        id = "wideswrd/action",
        blade = parts.blade,
        hit = { region = regions.wide, collision = parts.slashes_navis, hit_mod = 3 },
        effect = WIDE_SLASH,
        sound = parts.swing,
        charged = CHARGED,
    },
}

return { wideswrd = chip }
