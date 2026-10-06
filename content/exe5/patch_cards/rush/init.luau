--!strict
-- The patch card Rush (60, ラッシュ in the Japanese games), 11 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 11,
    effects = {
        cards.rush(true),
        cards.bug(cards.beat(false)),
        cards.bug(cards.tango(false)),
    },
}

return { rush = card }
