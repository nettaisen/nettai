--!strict
-- The patch card Miney (4, ジーラ in the Japanese games), 12 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 12,
    effects = {
        cards.attack(3),
        cards.bug(cards.float_shoes(false)),
        cards.bug(cards.air_shoes(false)),
    },
}

return { miney = card }
