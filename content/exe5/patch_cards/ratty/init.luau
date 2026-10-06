--!strict
-- The patch card Ratty (7, チュートン in the Japanese games), 10 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 10,
    effects = {
        cards.bug(cards.hp_percent(-20)),
        cards.gauge("fast"),
    },
}

return { ratty = card }
