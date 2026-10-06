--!strict
-- The patch card Dominerd (15, カーズ in the Japanese games), 18 MB.

local cards = require("../rules/patch_cards/cards")
local chips = require("../navis/megaman/weapons/chips")

local card: PatchCard = {
    mb = 18,
    effects = {
        cards.back_special(chips.crsshld),
        cards.bug(cards.hp(-50)),
    },
}

return { dominerd = card }
