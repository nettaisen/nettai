--!strict
-- The patch card SlashMan (48, スラッシュマン in the Japanese games), 31 MB.

local cards = require("../rules/patch_cards/cards")
local charge = require("@self/charge")

local card: PatchCard = {
    mb = 31,
    effects = {
        cards.attack(4),
        cards.charged_shot(charge),
        cards.bug(cards.buster_blanks(4, 1)),
    },
}

return { slashman = card }
