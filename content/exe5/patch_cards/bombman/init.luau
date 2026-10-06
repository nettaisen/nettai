--!strict
-- The patch card BombMan (84, ボンバーマン in the Japanese games), 43 MB.

local cards = require("../rules/patch_cards/cards")
local chips = require("../navis/megaman/weapons/chips")

local card: PatchCard = {
    mb = 43,
    effects = {
        cards.hp(150),
        cards.body("fire"),
        cards.bug(cards.custom(-1)),
        cards.charged_shot(chips.blkbomb),
    },
}

return { bombman = card }
