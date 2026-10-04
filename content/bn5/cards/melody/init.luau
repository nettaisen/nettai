--!strict
-- The patch card Melody (25, プルメロ in the Japanese games), 15 MB.

local cards = require("../rules/patch-cards/cards")

return define.patch_card {
    id = "melody",
    mb = 15,
    effects = {
        cards.hp(90),
        cards.step_bug(0),
        cards.bug(cards.super_armor(false)),
    },
}
