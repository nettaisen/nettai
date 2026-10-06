--!strict
-- The patch card KilPlant (7, キルプラント in the Japanese games), 19 MB.

local cards = require("../rules/patch_cards/cards")
local charge = require("@self/charge")

local card: PatchCard = {
    mb = 19,
    effects = {
        cards.body("wood"),
        cards.bug(cards.air_shoes(false)),
        cards.charged_shot(charge),
    },
}

return { kilplant = card }
