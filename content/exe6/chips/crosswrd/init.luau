--!strict
-- CrosSwrd: an X-shaped slash ahead, and a second hit on the panel ahead.

local lockon = require("../rules/lockon")
local parts = require("../lib/swords/parts")
local regions = require("@exelib/regions")
local slash = require("../lib/swords/slash")
local slashes = require("../navis/megaman/forms/slashcross/slashes")

-- CrosSwrd's X.
local CROSS_SLASH = new.effect { sprite = asset.sprite("cross-slash"), anim = 0, palette = 0 }

-- Its charged slash in SlashCross: its X, and the plain sword's wave unseen under it.
local CHARGED = slashes.slash {
    wave = slashes.wave { region = regions.x, sprite = asset.sprite("cross-slash"), anim = 0, animates = true },
    hidden_wave = slashes.wave { region = regions.single, sprite = asset.sprite("sword-slash"), anim = 2 },
    blade = parts.blade,
    sound = parts.swing,
    lockon = lockon.thunder,
}

local chip: Chip = {
    codes = { "E", "L", "S", "*" },
    element = "null", family = "sword", class = "special",
    rarity = 0, mb = 8, damage = 80, hit_param = 30, lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
    -- The Beast rush, and its lock-on mode (rules/lockon).
    beast = { lockon = lockon.crosswrd },
    action = slash.action {
        blade = parts.blade,
        hit = { region = regions.x, collision = parts.slashes_navis, hit_mod = 3 },
        effect = CROSS_SLASH,
        sound = parts.swing,
        second_hit = slash.CROSS_HIT,
        charged = CHARGED,
    },
}

return { crosswrd = chip }
