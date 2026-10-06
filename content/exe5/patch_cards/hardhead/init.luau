--!strict
-- The patch card HardHead (54, ハルドボルズ in the Japanese games), 10 MB.

local cards = require("../rules/patch_cards/cards")
local chips = require("../navis/megaman/weapons/chips")

local card: PatchCard = {
    mb = 10,
    effects = {
        cards.charged_shot(chips.cannball),
        cards.emotion_bug(0),
    },
}

return { hardhead = card }
