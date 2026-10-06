--!strict
-- The patch card BrushMan (17, フデロー in the Japanese games), 19 MB.

local cards = require("../rules/patch_cards/cards")
local variants = require("../objects/projectile/variants")

local card: PatchCard = {
    mb = 19,
    effects = {
        cards.bug(cards.hp_percent(-30)),
        cards.buster_shot(variants.poisoning),
        cards.panel_trail("holy", 1),
    },
}

return { brushman = card }
