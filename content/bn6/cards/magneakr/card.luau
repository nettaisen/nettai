--!strict
-- The patch card Magneakr (63, マグニッカー in the Japanese games), 7 MB.

local cards = require("../../rules/patch-cards/cards")
local variants = require("../../objects/projectile/variants")

return define.patch_card {
    id = "magneakr",
    mb = 7,
    effects = {
        cards.hp(30),
        cards.body("elec"),
        cards.bug(cards.buster_shot(nil)),
        cards.charge_shot_kind(variants.charged_road_back),
    },
}
