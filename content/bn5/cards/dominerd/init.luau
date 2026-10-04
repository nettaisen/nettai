--!strict
-- The patch card Dominerd (15, カーズ in the Japanese games), 18 MB.

local cards = require("../rules/patch-cards/cards")
local chips = require("../navis/megaman/weapons/chips")

return define.patch_card {
    id = "dominerd",
    mb = 18,
    effects = {
        cards.back_special(chips.crsshld),
        cards.bug(cards.hp(-50)),
    },
}
