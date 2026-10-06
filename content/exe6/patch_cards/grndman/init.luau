--!strict
-- The patch card GrndMan (50, グランドマン in the Japanese games), 43 MB.

local cards = require("../rules/patch_cards/cards")
local charge = require("@self/charge")

local card: PatchCard = {
    mb = 43,
    effects = {
        cards.hp_percent(25),
        cards.charged_shot(charge),
        cards.bug(cards.custom(-1)),
        cards.bug(cards.mega_folder(-1)),
    },
}

return { grndman = card }
