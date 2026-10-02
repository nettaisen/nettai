--!strict
-- The patch card Slimey (12, ジェライム in the Japanese games), 12 MB.

local cards = require("../../rules/patch-cards/cards")
local charge = require("./charge")

return cards.card {
    id = "patch-card/slimey",
    name = "Slimey",
    mb = 12,
    effects = {
        cards.hp(30),
        cards.body("aqua"),
        cards.charged_shot(charge),
        cards.bug(cards.gauge("normal")),
    },
}
