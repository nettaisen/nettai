--!strict
-- The patch card Miney (4, ジーラ in the Japanese games), 12 MB.

local cards = require("../../rules/patch-cards/cards")

return cards.card {
    id = "patch-card/miney",
    name = "Miney",
    mb = 12,
    effects = {
        cards.bug(cards.air_shoes(false)),
        cards.bug(cards.float_shoes(false)),
        cards.attack(3),
    },
}
