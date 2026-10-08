--!strict
-- WideBlde: WideSwrd's slash, stronger.

local lockon = require("../rules/lockon")
local parts = require("../lib/swords/parts")
local regions = require("@exelib/regions")
local slash = require("../lib/swords/slash")
local slashes = require("../navis/megaman/forms/slashcross/slashes")

-- WideBlde's slash.
local WIDE_BLADE_SLASH = new.effect { sprite = asset.sprite("sword-slash"), anim = 0, palette = 5 }

-- Its charged slash in SlashCross: over a column, in palette 5.
local CHARGED = slashes.slash {
    wave = slashes.wave { region = regions.wide, sprite = asset.sprite("sword-slash"), anim = 0, palette = 5 },
    blade = parts.blade,
    sound = parts.swing,
    lockon = lockon.widesht,
}

local chip: Chip = {
    codes = { "B", "R", "W" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 3,
    mb = 38,
    damage = 150,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    beast = { lockon = lockon.widesht },
    slot_in_limit = 3,
    action = slash.action {
        id = "wideblde/action",
        blade = parts.blade,
        hit = { region = regions.wide, collision = parts.slashes_navis, hit_mod = 3 },
        effect = WIDE_BLADE_SLASH,
        sound = parts.swing,
        charged = CHARGED,
    },
}

return { wideblde = chip }
