--!strict
-- The patch card Catack (27, キャタック in the Japanese games), 20 MB.

local cards = require("../../rules/patch-cards/cards")

return define.patch_card {
    id = "bn5:catack",
    mb = 20,
    effects = {
        cards.hp(100),
        cards.super_armor(true),
        cards.bug(cards.air_shoes(false)),
    },
}
