--!strict
-- The patch card MagntMan (38, マグネットマン in the Japanese games), 37 MB.

local cards = require("../../rules/patch-cards/cards")
local back_special = require("./back-special")
local variants = require("../../objects/projectile/variants")

return cards.card {
    id = "patch-card/magntman",
    name = "MagntMan",
    mb = 37,
    effects = {
        cards.hp(240),
        cards.body("elec"),
        cards.bug(cards.charge_shot_kind(variants.charged)),
        cards.back_special(back_special),
    },
}
