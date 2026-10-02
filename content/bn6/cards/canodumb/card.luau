--!strict
-- The patch card Canodumb (1, キャノーダム in the Japanese games), 10 MB.

local cards = require("../../rules/patch-cards/cards")
local charge = require("./charge")

return cards.card {
    id = "patch-card/canodumb",
    mb = 10,
    effects = {
        cards.bug(cards.hp(-40)),
        cards.attack(1),
        cards.charged_shot(charge),
    },
}
