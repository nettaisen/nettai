--!strict
-- The patch card GyroMan (43, ジャイロマン in the Japanese games), 45 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 45,
    effects = {
        cards.float_shoes(true),
        cards.air_shoes(true),
        cards.beat(true),
        cards.bug(cards.hp_percent(-5)),
    },
}

return { gyroman = card }
