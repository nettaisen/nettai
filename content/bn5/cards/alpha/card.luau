--!strict
-- The patch card Alpha (104, プロト in the Japanese games), 55 MB.

local cards = require("../../rules/patch-cards/cards")

return define.patch_card {
    id = "bn5:alpha",
    mb = 55,
    effects = {
        cards.hp(100),
        cards.giga_folder(2),
        cards.bug(cards.custom(-3)),
    },
}
