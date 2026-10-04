--!strict
-- The patch card BombMan (84, ボンバーマン in the Japanese games), 43 MB.

local cards = require("../../rules/patch-cards/cards")
local chips = require("../../navis/megaman/weapons/chips")

return define.patch_card {
    id = "bombman",
    mb = 43,
    effects = {
        cards.hp(150),
        cards.body("fire"),
        cards.charged_shot(chips.blkbomb),
        cards.bug(cards.custom(-1)),
    },
}
