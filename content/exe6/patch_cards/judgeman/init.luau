--!strict
-- The patch card JudgeMan (105, ジャッジマン in the Japanese games), 38 MB.

local cards = require("../rules/patch_cards/cards")
local charge = require("@self/charge")

local card: PatchCard = {
    mb = 38,
    effects = {
        cards.bug(cards.hp_percent(-10)),
        cards.custom(2),
        cards.charged_shot(charge),
        cards.bug(cards.gauge("normal")),
    },
}

return { judgeman = card }
