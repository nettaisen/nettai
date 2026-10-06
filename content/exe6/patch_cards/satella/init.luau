--!strict
-- The patch card Satella (58, サテラ in the Japanese games), 8 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 8,
    effects = {
        cards.rapid(4),
        cards.air_shoes(true),
        cards.bug(cards.custom_damage(20)),
    },
}

return { satella = card }
