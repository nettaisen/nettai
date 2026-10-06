--!strict
-- The patch card BowlMan (94, ボウルマン in the Japanese games), 30 MB.

local cards = require("../rules/patch_cards/cards")
local charge = require("@self/charge")

local card: PatchCard = {
    mb = 30,
    effects = {
        cards.hp(100),
        cards.bug(cards.float_shoes(false)),
        cards.charged_shot(charge),
        cards.custom(1),
    },
}

return { bowlman = card }
