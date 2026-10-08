--!strict
-- StepSwrd: a step two panels ahead, a slash on the column ahead, and back. (ProtoMan's own copy is
-- protoman.luau.)

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
    codes = { "B", "L", "P" },
    element = "null", family = "sword", class = "standard",
    rarity = 2, mb = 28, damage = 160, hit_param = 20, lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    -- The Beast rush, and its lock-on mode (rules/lockon).
    beast = { lockon = lockon.gundelex },
    action = slash.action {
        blade = parts.blade,
        hit = { region = regions.wide, collision = parts.slashes_navis, hit_mod = 3 },
        effect = WIDE_SLASH,
        sound = parts.swing,
        step = true,
        charged = CHARGED,
    },
}

return { stepswrd = chip }
