--!strict
-- The patch card VolGear (2, ボルケルギア in the Japanese games), 12 MB.

local cards = require("../rules/patch_cards/cards")

return define.patch_card {
    id = "volgear",
    mb = 12,
    effects = {
        cards.hp(60),
        cards.body("fire"),
        cards.attack(1),
        cards.buster_blanks(0, 0),
    },
}
