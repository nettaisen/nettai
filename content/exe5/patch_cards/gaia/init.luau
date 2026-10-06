--!strict
-- The patch card Gaia (56, ガイアント in the Japanese games), 15 MB.

local cards = require("../rules/patch_cards/cards")
local variants = require("../objects/projectile/variants")

local card: PatchCard = {
    mb = 15,
    effects = {
        cards.hp(100),
        cards.bug(cards.float_shoes(false)),
        cards.bug(cards.air_shoes(false)),
        cards.buster_shot(variants.cracking),
    },
}

return { gaia = card }
