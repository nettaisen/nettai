--!strict
-- The patch card Satella (58, サテラ in the Japanese games), 8 MB.

local cards = require("../../rules/patch-cards/cards")

return cards.card {
    id = "patch-card/satella",
    mb = 8,
    effects = {
        cards.air_shoes(true),
        cards.rapid(4),
        cards.bug(cards.custom_damage(20)),
    },
}
