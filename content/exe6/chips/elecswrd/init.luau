--!strict
-- ElecSwrd: an elec slash on the column ahead that paralyzes.

local attachment = require("../objects/attachment")
local lockon = require("../rules/lockon")
local parts = require("../lib/swords/parts")
local regions = require("@exelib/regions")
local slash = require("../lib/swords/slash")
local slashes = require("../navis/megaman/forms/slashcross/slashes")
local statuses = require("../rules/status")

-- The slash over a column.
local WIDE_SLASH = new.effect { sprite = asset.sprite("sword-slash"), anim = 0, palette = 0 }
-- ElecSwrd's blade (row 0x1B).
local ELEC_BLADE = attachment.look { sprite = asset.sprite("elec-sword"), palette = 0 }

-- Its charged slash in SlashCross: over a column, in palette 3, its own blade; it paralyzes.
local CHARGED = slashes.slash {
    wave = slashes.wave {
        region = regions.wide,
        sprite = asset.sprite("sword-slash"),
        anim = 0,
        palette = 3,
        hit_mod = 1,
        status = statuses.paralyze_90,
    },
    blade = ELEC_BLADE,
    sound = parts.swing,
    lockon = lockon.widesht,
}

local chip: Chip = {
    codes = { "E", "K", "N" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 2,
    mb = 35,
    damage = 120,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    beast = { lockon = lockon.widesht },
    slot_in_limit = 3,
    traits = { "element_sword" },
    action = slash.action {
        id = "elecswrd/action",
        blade = ELEC_BLADE,
        hit = { region = regions.wide, collision = parts.slashes_navis, hit_mod = 1, status = statuses.paralyze_90 },
        effect = WIDE_SLASH,
        effect_palette = 3,
        sound = parts.swing,
        charged = CHARGED,
    },
}

return { elecswrd = chip }
