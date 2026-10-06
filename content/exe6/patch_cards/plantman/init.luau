--!strict
-- The patch card PlantMan (92, プラントマン in the Japanese games), 42 MB.

local cards = require("../rules/patch_cards/cards")
local variants = require("../objects/projectile/variants")

local card: PatchCard = {
    mb = 42,
    effects = {
        cards.bug(cards.hp(-80)),
        cards.body("wood"),
        cards.status_guard(true),
        cards.charge_shot_kind(variants.charged_grass),
        cards.mega_folder(2),
    },
}

return { plantman = card }
