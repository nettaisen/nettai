--!strict
-- The patch card TenguMan (103, テングマン in the Japanese games), 34 MB.

local cards = require("../../rules/patch-cards/cards")
local charge = require("./charge")

return cards.card {
    id = "patch-card/tenguman",
    name = "TenguMan",
    mb = 34,
    effects = {
        cards.air_shoes(true),
        cards.bug(cards.attack(-1)),
        cards.charged_shot(charge),
        cards.beat(true),
    },
}
