--!strict
-- The patch card BeastMan (40, ビーストマン in the Japanese games), 33 MB.

local cards = require("../../rules/patch-cards/cards")
local charge = require("./charge")

return cards.card {
    id = "patch-card/beastman",
    mb = 33,
    effects = {
        cards.undershirt(true),
        cards.rapid(4),
        cards.charged_shot(charge),
        cards.rush(true),
        cards.bug(cards.step_bug(1)),
    },
}
