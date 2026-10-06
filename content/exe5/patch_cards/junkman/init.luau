--!strict
-- The patch card JunkMan (42, ジャンクマン in the Japanese games), 40 MB.

local cards = require("../rules/patch_cards/cards")
local chips = require("../navis/megaman/weapons/chips")

local card: PatchCard = {
    mb = 40,
    effects = {
        cards.hp(150),
        cards.charged_shot(chips.poltrgst),
        cards.bug(cards.buster_blanks(4, 4)),
    },
}

return { junkman = card }
