--!strict
-- The patch card Dharma (74, ダーマ in the Japanese games), 22 MB.

local cards = require("../rules/patch-cards/cards")
local chips = require("../navis/megaman/weapons/chips")

return define.patch_card {
    id = "dharma",
    mb = 22,
    effects = {
        cards.undershirt(true),
        cards.charged_shot(chips.airhoc),
        cards.bug(cards.hp_percent(-15)),
    },
}
