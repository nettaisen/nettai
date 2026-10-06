--!strict
-- The patch card QuickMan (35, クイックマン in the Japanese games), 38 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 38,
    effects = {
        cards.bug(cards.hp_percent(-10)),
        cards.rapid(5),
        cards.charge(5),
        cards.gauge("fast"),
    },
}

return { quickman = card }
