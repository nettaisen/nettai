--!strict
-- The patch card Batty (79, パルスバット in the Japanese games), 25 MB.

local cards = require("../rules/patch-cards/cards")
local chips = require("../navis/megaman/weapons/chips")

return define.patch_card {
    id = "batty",
    mb = 25,
    effects = {
        cards.air_shoes(true),
        cards.charged_shot(chips.pulsar),
        cards.bug(cards.hp_percent(-5)),
    },
}
