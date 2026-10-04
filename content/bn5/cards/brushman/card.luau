--!strict
-- The patch card BrushMan (17, フデロー in the Japanese games), 19 MB.

local cards = require("../../rules/patch-cards/cards")
local variants = require("../../objects/projectile/variants")

return define.patch_card {
    id = "brushman",
    mb = 19,
    effects = {
        cards.buster_shot(variants.poisoning),
        cards.panel_trail("holy", 1),
        cards.bug(cards.hp_percent(-30)),
    },
}
