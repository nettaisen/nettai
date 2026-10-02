--!strict
-- The patch card HonyBomr (80, ハニホー in the Japanese games), 12 MB.

local cards = require("../../rules/patch-cards")
local back_special = require("./back-special")

return define.patch_card {
    id = "honybomr",
    name = "HonyBomr",
    mb = 12,
    effects = {
        cards.bug(cards.hp_percent(-10)),
        cards.body("wood"),
        cards.air_shoes(true),
        cards.back_special(back_special),
    },
}
