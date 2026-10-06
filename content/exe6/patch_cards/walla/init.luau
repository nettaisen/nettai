--!strict
-- The patch card Walla (68, ウォーラ in the Japanese games), 12 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 12,
    effects = {
        cards.hp(60),
        cards.bug(cards.first_barrier(nil)),
        cards.custom(1),
    },
}

return { walla = card }
