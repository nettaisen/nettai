--!strict
-- The patch card ColdMan (98, コールドマン in the Japanese games), 34 MB.

local cards = require("../../rules/patch-cards")
local variants = require("../../objects/projectile/variants")

return define.patch_card {
    id = "coldman",
    name = "ColdMan",
    mb = 34,
    effects = {
        cards.body("aqua"),
        cards.bug(cards.attack(-1)),
        cards.charge_shot_kind(variants.charged_freezing),
        cards.bug(cards.panel_trail("ice", 1)),
    },
}
