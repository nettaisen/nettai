--!strict
-- The patch card Puffy (55, プクール in the Japanese games), 12 MB.

local cards = require("../rules/patch_cards/cards")
local charge = require("@self/charge")

local card: PatchCard = {
    mb = 12,
    effects = {
        cards.body("aqua"),
        cards.float_shoes(true),
        cards.charged_shot(charge),
        cards.bug(cards.back_special(nil)),
    },
}

return { puffy = card }
