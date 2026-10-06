--!strict
-- The patch card PulsBulb (77, パルフォロン in the Japanese games), 13 MB.

local cards = require("../rules/patch_cards/cards")
local charge = require("@self/charge")

local card: PatchCard = {
    mb = 13,
    effects = {
        cards.body("elec"),
        cards.charged_shot(charge),
        cards.bug(cards.emotion_bug(1)),
    },
}

return { pulsbulb = card }
