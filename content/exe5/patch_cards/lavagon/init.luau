--!strict
-- The patch card Lavagon (66, マグマドラゴン in the Japanese games), 20 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 20,
    effects = {
        cards.hp(150),
        cards.body("fire"),
        cards.air_shoes(true),
        cards.bug(cards.first_barrier(nil)),
        cards.bug(cards.panel_trail("hole", 2)),
    },
}

return { lavagon = card }
