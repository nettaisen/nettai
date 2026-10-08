--!strict
-- BambSwrd: a wood slash on the column ahead.

local attachment = require("../objects/attachment")
local lockon = require("../rules/lockon")
local parts = require("../lib/swords/parts")
local regions = require("@exelib/regions")
local slash = require("../lib/swords/slash")
local slashes = require("../navis/megaman/forms/slashcross/slashes")

-- The slash over a column.
local WIDE_SLASH = new.effect { sprite = asset.sprite("sword-slash"), anim = 0, palette = 0 }
-- BambSwrd's blade (row 0x1C: the sword in palette 2).
local BAMBOO_BLADE = attachment.look { sprite = asset.sprite("sword"), palette = 2 }

-- Its charged slash in SlashCross: over a column, in palette 4, its own blade.
local CHARGED = slashes.slash {
    wave = slashes.wave {
        region = regions.wide,
        sprite = asset.sprite("sword-slash"),
        anim = 0,
        palette = 4,
        hit_mod = 1,
    },
    blade = BAMBOO_BLADE,
    sound = parts.swing,
    lockon = lockon.widesht,
}

local chip: Chip = {
    codes = { "H", "S", "W" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 2,
    mb = 34,
    damage = 150,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    beast = { lockon = lockon.widesht },
    slot_in_limit = 3,
    traits = { "element_sword" },
    action = slash.action {
        id = "bambswrd/action",
        blade = BAMBOO_BLADE,
        hit = { region = regions.wide, collision = parts.slashes_navis, hit_mod = 1 },
        effect = WIDE_SLASH,
        effect_palette = 4,
        sound = parts.swing,
        charged = CHARGED,
    },
}

return { bambswrd = chip }
