--!strict
-- The patch card Dharma (74, ダーマ in the Japanese games), 22 MB.

local cards = require("../rules/patch_cards/cards")
local chips = require("../navis/megaman/weapons/chips")

local card: PatchCard = {
    mb = 22,
    effects = {
        cards.bug(cards.hp_percent(-15)),
        cards.undershirt(true),
        cards.charged_shot(chips.airhoc),
    },
}

return { dharma = card }
