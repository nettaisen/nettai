--!strict
-- The patch card JudgeMan (105, ジャッジマン in the Japanese games), 38 MB.

local cards = require("../../rules/patch-cards/cards")
local charge = require("./charge")

return define.patch_card {
    id = "bn6:judgeman",
    mb = 38,
    effects = {
        cards.bug(cards.hp_percent(-10)),
        cards.charged_shot(charge),
        cards.bug(cards.gauge("normal")),
        cards.custom(2),
    },
}
