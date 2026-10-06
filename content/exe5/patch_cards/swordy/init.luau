--!strict
-- The patch card Swordy (4, スウォーディン in the Japanese games), 10 MB.

local cards = require("../rules/patch_cards/cards")
local chips = require("../navis/megaman/weapons/chips")

local card: PatchCard = {
    mb = 10,
    effects = {
        cards.charged_shot(chips.longswrd),
        cards.bug(cards.attack(-1)),
        cards.bug(cards.rapid(-1)),
        cards.bug(cards.charge(-1)),
    },
}

return { swordy = card }
