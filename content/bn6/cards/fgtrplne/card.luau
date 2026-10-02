--!strict
-- The patch card FgtrPlne (26, ゼロプレーン in the Japanese games), 20 MB.

local cards = require("../../rules/patch-cards/cards")

return cards.card {
    id = "patch-card/fgtrplne",
    name = "FgtrPlne",
    mb = 20,
    effects = {
        cards.bug(cards.hp_percent(-10)),
        cards.float_shoes(true),
        cards.air_shoes(true),
    },
}
