--!strict
-- The patch card Walla (68, ウォーラ in the Japanese games), 12 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 12,
    effects = {
        cards.hp(60),
        cards.custom(1),
        cards.bug(cards.first_barrier(nil)),
    },
}

return { walla = card }
