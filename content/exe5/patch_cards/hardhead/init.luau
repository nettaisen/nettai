--!strict
-- The patch card HardHead (54, ハルドボルズ in the Japanese games), 10 MB.

local cards = require("../rules/patch_cards/cards")
local chips = require("../navis/megaman/weapons/chips")

return define.patch_card {
    id = "hardhead",
    mb = 10,
    effects = {
        cards.charged_shot(chips.cannball),
        cards.emotion_bug(0),
    },
}
