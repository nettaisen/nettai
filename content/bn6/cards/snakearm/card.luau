--!strict
-- The patch card SnakeArm (82, スナーム in the Japanese games), 11 MB.

local cards = require("../../rules/patch-cards/cards")
local charge = require("./charge")

return cards.card {
    id = "patch-card/snakearm",
    name = "SnakeArm",
    mb = 11,
    effects = {
        cards.attack(3),
        cards.bug(cards.rapid(-1)),
        cards.bug(cards.charge(-1)),
        cards.charged_shot(charge),
    },
}
