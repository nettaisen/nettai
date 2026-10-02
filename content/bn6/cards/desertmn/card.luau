--!strict
-- The patch card DesertMn (41, デザートマン in the Japanese games), 36 MB.

local cards = require("../../rules/patch-cards/cards")

return cards.card {
    id = "patch-card/desertmn",
    name = "DesertMn",
    mb = 36,
    effects = {
        cards.hp(160),
        cards.bug(cards.air_shoes(false)),
        cards.charge(4),
        cards.hit_status("none"),
    },
}
