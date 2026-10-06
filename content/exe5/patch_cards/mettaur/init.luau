--!strict
-- The patch card Mettaur (1, メットール in the Japanese games), 7 MB.

local cards = require("../rules/patch_cards/cards")
local chips = require("../navis/megaman/weapons/chips")

return define.patch_card {
    id = "mettaur",
    mb = 7,
    effects = {
        cards.hp(40),
        cards.back_special(chips.mettguard),
        cards.bug(cards.float_shoes(false)),
    },
}
