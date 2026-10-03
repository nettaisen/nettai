--!strict
-- The patch card Shrubby (79, モリキュー in the Japanese games), 6 MB.

local cards = require("../../rules/patch-cards/cards")
local chrgs = require("../../lib/patch-cards/chrgs")
local variants = require("../../objects/projectile/variants")

return define.patch_card {
    id = "bn6:shrubby",
    mb = 6,
    effects = {
        cards.body("wood"),
        cards.buster_shot(variants.grass),
        cards.bug(cards.charged_shot(chrgs)),
        cards.custom_damage(0),
    },
}
