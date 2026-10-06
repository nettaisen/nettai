--!strict
-- The patch card Batty (79, パルスバット in the Japanese games), 25 MB.

local cards = require("../rules/patch_cards/cards")
local chips = require("../navis/megaman/weapons/chips")

local card: PatchCard = {
    mb = 25,
    effects = {
        cards.bug(cards.hp_percent(-5)),
        cards.air_shoes(true),
        cards.charged_shot(chips.pulsar),
    },
}

return { batty = card }
