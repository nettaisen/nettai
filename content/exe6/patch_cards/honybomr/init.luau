--!strict
-- The patch card HonyBomr (80, ハニホー in the Japanese games), 12 MB.

local cards = require("../rules/patch_cards/cards")
local back_special = require("@self/back_special")

local card: PatchCard = {
    mb = 12,
    effects = {
        cards.bug(cards.hp_percent(-10)),
        cards.body("wood"),
        cards.air_shoes(true),
        cards.back_special(back_special),
    },
}

return { honybomr = card }
