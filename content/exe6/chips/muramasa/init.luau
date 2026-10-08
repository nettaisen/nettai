--!strict
-- Muramasa: a slash on the two panels ahead, as strong as the HP its user lost.

local lockon = require("../rules/lockon")
local parts = require("../lib/swords/parts")
local regions = require("@exelib/regions")
local slash = require("../lib/swords/slash")
local slashes = require("../navis/megaman/forms/slashcross/slashes")

-- Muramasa's slash.
local MURAMASA_SLASH = new.effect { sprite = asset.sprite("sword-slash"), anim = 1, palette = 6 }

-- Its charged slash in SlashCross: over two panels, in palette 6.
local CHARGED = slashes.slash {
    wave = slashes.wave { region = regions.long, sprite = asset.sprite("sword-slash"), anim = 1, palette = 6 },
    blade = parts.blade,
    sound = parts.swing,
    lockon = lockon.firebrn1,
}

local chip: Chip = {
    codes = { "M" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 3,
    mb = 77,
    damage = { formula = "hp_lost" },
    hit_param = 50,
    lockout = 0,
    flags = { "has_damage", "standard_library", "damage_shown_variable", "library", "variable_damage" },
    -- (The custom screen shows its damage as "???" only on an A, which it
    -- never comes as.)
    traits = { "hides_damage_as_a" },
    beast = { lockon = lockon.firebrn1 },
    slot_in_limit = 1,
    action = slash.action {
        id = "muramasa/action",
        blade = parts.blade,
        hit = { region = regions.long, collision = parts.slashes_navis, hit_mod = 3 },
        effect = MURAMASA_SLASH,
        sound = parts.swing,
        charged = CHARGED,
    },
}

return { muramasa = chip }
