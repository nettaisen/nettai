--!strict
-- The patch card KendoMan (97, ケンドーマン in the Japanese games), 38 MB.

local cards = require("../../rules/patch-cards/cards")
local charge = require("./charge")

return cards.card {
    id = "patch-card/kendoman",
    name = "KendoMan",
    mb = 38,
    effects = {
        cards.hp_percent(20),
        cards.charged_shot(charge),
        cards.bug(cards.buster_blanks(4, 1)),
    },
}
