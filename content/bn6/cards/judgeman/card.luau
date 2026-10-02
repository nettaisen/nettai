--!strict
-- The patch card JudgeMan (105, ジャッジマン in the Japanese games), 38 MB.

local cards = require("../../rules/patch-cards")
local charge = require("./charge")

return define.patch_card {
    id = "judgeman",
    name = "JudgeMan",
    mb = 38,
    effects = {
        cards.bug(cards.hp_percent(-10)),
        cards.charged_shot(charge),
        cards.bug(cards.gauge("normal")),
        cards.custom(2),
    },
}
