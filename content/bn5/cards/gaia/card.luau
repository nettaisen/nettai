--!strict
-- The patch card Gaia (56, ガイアント in the Japanese games), 15 MB.

local cards = require("../../rules/patch-cards/cards")
local variants = require("../../objects/projectile/variants")

return define.patch_card {
    id = "gaia",
    mb = 15,
    effects = {
        cards.hp(100),
        cards.buster_shot(variants.cracking),
        cards.bug(cards.float_shoes(false)),
        cards.bug(cards.air_shoes(false)),
    },
}
