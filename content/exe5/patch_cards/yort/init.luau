--!strict
-- The patch card Yort (14, ヨーヨット in the Japanese games), 17 MB.

local cards = require("../rules/patch_cards/cards")
local chips = require("../navis/megaman/weapons/chips")

local card: PatchCard = {
    mb = 17,
    effects = {
        cards.hp(30),
        cards.charged_shot(chips.yoyo),
        cards.bug(cards.back_special(nil)),
    },
}

return { yort = card }
