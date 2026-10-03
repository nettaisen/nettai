--!strict
-- The patch card SnakeArm (82, スナーム in the Japanese games), 11 MB.

local cards = require("../../rules/patch-cards/cards")
local charge = require("./charge")

return define.patch_card {
    id = "bn6:snakearm",
    mb = 11,
    effects = {
        cards.attack(3),
        cards.bug(cards.rapid(-1)),
        cards.bug(cards.charge(-1)),
        cards.charged_shot(charge),
    },
}
