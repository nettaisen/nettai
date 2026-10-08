--!strict
-- FtrSword: a slash on the three panels ahead.

local lockon = require("../rules/lockon")
local parts = require("../lib/swords/parts")
local regions = require("@exelib/regions")
local slash = require("../lib/swords/slash")
local slashes = require("../navis/megaman/forms/slashcross/slashes")

-- The slash over three panels.
local LINE_SLASH = new.effect { sprite = asset.sprite("sword-slash"), anim = 3, palette = 0 }

-- Its charged slash in SlashCross: over three panels.
local CHARGED = slashes.slash {
    wave = slashes.wave { region = regions.line, sprite = asset.sprite("sword-slash"), anim = 3 },
    blade = parts.blade,
    sound = parts.swing,
    lockon = lockon.yoyo,
}

local chip: Chip = {
    codes = { "E", "L", "S", "*" },
    element = "null", family = "sword", class = "special",
    rarity = 0, mb = 8, damage = 80, hit_param = 30, lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    -- The Beast rush, and its lock-on mode (rules/lockon).
    beast = { lockon = lockon.yoyo },
    action = slash.action {
        blade = parts.blade,
        hit = { region = regions.line, collision = parts.slashes_navis, hit_mod = 3 },
        effect = LINE_SLASH,
        sound = parts.swing,
        charged = CHARGED,
    },
}

return { ftrsword = chip }
