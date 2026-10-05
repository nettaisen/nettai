--!strict
-- The patch card TenguMan (103, テングマン in the Japanese games), 34 MB.

local cards = require("../rules/patch-cards/cards")
local charge = require("@self/charge")

return define.patch_card {
    id = "tenguman",
    mb = 34,
    effects = {
        cards.air_shoes(true),
        cards.bug(cards.attack(-1)),
        cards.charged_shot(charge),
        cards.beat(true),
    },
}
