--!strict
-- The patch card ThunMan (90, サンダーマン in the Japanese games), 36 MB.

local cards = require("../../rules/patch-cards/cards")
local charge = require("./charge")

return cards.card {
    id = "patch-card/thunman",
    name = "ThunMan",
    mb = 36,
    effects = {
        cards.body("elec"),
        cards.charged_shot(charge),
        cards.chip_recovery(10),
        cards.bug(cards.custom(-1)),
        cards.bug(cards.hp_drain(1)),
    },
}
