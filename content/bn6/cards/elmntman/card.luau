--!strict
-- The patch card ElmntMan (106, エレメントマン in the Japanese games), 33 MB.

local cards = require("../../rules/patch-cards/cards")
local chrgs = require("../../lib/patch-cards/chrgs")
local variants = require("../../objects/projectile/variants")

return cards.card {
    id = "patch-card/elmntman",
    mb = 33,
    effects = {
        cards.hp(150),
        cards.buster_shot(variants.grass),
        cards.charge_shot_kind(variants.charged_ice),
        cards.bug(cards.charged_shot(chrgs)),
        cards.panel_trail("none", 0),
    },
}
