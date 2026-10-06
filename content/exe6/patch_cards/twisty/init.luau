--!strict
-- The patch card Twisty (59, パララ& リモコゴロー in the Japanese games), 9 MB.

local cards = require("../rules/patch_cards/cards")
local variants = require("../objects/projectile/variants")

local card: PatchCard = {
    mb = 9,
    effects = {
        cards.hp(80),
        cards.body("elec"),
        cards.bug(cards.air_shoes(false)),
        cards.buster_shot(variants.cracking),
        cards.bug(cards.panel_trail("hole", 1)),
    },
}

return { twisty = card }
