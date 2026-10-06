--!strict
-- The patch card DesertMn (41, デザートマン in the Japanese games), 36 MB.

local cards = require("../rules/patch_cards/cards")

return define.patch_card {
    id = "desertmn",
    mb = 36,
    effects = {
        cards.hp(160),
        cards.bug(cards.air_shoes(false)),
        cards.charge(4),
        cards.hit_status("none"),
    },
}
