--!strict
-- The patch card SparkMan (41, スパークマン in the Japanese games), 43 MB.

local cards = require("../rules/patch-cards/cards")
local variants = require("../objects/projectile/variants")

return define.patch_card {
    id = "sparkman",
    mb = 43,
    effects = {
        cards.hp(140),
        cards.body("elec"),
        cards.charge_shot_kind(variants.charged_blinding),
        cards.bug(cards.gauge("normal")),
    },
}
