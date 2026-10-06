--!strict
-- The patch card DesertMn (41, デザートマン in the Japanese games), 36 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 36,
    effects = {
        cards.hp(160),
        cards.charge(4),
        cards.bug(cards.air_shoes(false)),
        cards.hit_status("none"),
    },
}

return { desertmn = card }
