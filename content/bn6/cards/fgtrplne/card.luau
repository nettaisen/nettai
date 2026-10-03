--!strict
-- The patch card FgtrPlne (26, ゼロプレーン in the Japanese games), 20 MB.

local cards = require("../../rules/patch-cards/cards")

return define.patch_card {
    id = "bn6:fgtrplne",
    mb = 20,
    effects = {
        cards.bug(cards.hp_percent(-10)),
        cards.float_shoes(true),
        cards.air_shoes(true),
    },
}
