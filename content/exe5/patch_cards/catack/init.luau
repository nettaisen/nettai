--!strict
-- The patch card Catack (27, キャタック in the Japanese games), 20 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 20,
    effects = {
        cards.hp(100),
        cards.super_armor(true),
        cards.bug(cards.air_shoes(false)),
    },
}

return { catack = card }
