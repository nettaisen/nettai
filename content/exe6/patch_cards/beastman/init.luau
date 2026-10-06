--!strict
-- The patch card BeastMan (40, ビーストマン in the Japanese games), 33 MB.

local cards = require("../rules/patch_cards/cards")
local charge = require("@self/charge")

local card: PatchCard = {
    mb = 33,
    effects = {
        cards.rapid(4),
        cards.undershirt(true),
        cards.charged_shot(charge),
        cards.rush(true),
        cards.bug(cards.step_bug(1)),
    },
}

return { beastman = card }
