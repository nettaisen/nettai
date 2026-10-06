--!strict
-- The patch card Anaconda (10, ダイジャン in the Japanese games), 12 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 12,
    effects = {
        cards.hp(130),
        cards.body("wood"),
        cards.bug(cards.rush(false)),
        cards.bug(cards.beat(false)),
        cards.bug(cards.tango(false)),
    },
}

return { anaconda = card }
