--!strict
-- The patch card CloudMan (100, クラウドマン in the Japanese games), 40 MB.

local cards = require("../rules/patch_cards/cards")
local charge = require("@self/charge")

local card: PatchCard = {
    mb = 40,
    effects = {
        cards.hp(230),
        cards.body("elec"),
        cards.charged_shot(charge),
        cards.bug(cards.mega_folder(-1)),
    },
}

return { cloudman = card }
