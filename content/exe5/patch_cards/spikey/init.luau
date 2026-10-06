--!strict
-- The patch card Spikey (12, ガルー in the Japanese games), 6 MB.

local cards = require("../rules/patch_cards/cards")
local spreads = require("../navis/megaman/weapons/spreads")

local card: PatchCard = {
    mb = 6,
    effects = {
        cards.body("fire"),
        cards.charged_shot(spreads.heat),
        cards.bug(cards.rapid(-1)),
        cards.bug(cards.charge(-1)),
    },
}

return { spikey = card }
