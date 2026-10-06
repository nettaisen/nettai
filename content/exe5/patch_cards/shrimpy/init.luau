--!strict
-- The patch card Shrimpy (63, エビロン in the Japanese games), 10 MB.

local cards = require("../rules/patch_cards/cards")
local spreads = require("../navis/megaman/weapons/spreads")

local card: PatchCard = {
    mb = 10,
    effects = {
        cards.body("aqua"),
        cards.charged_shot(spreads.bubble),
        cards.bug(cards.attack(-1)),
    },
}

return { shrimpy = card }
