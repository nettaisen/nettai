--!strict
-- The patch card SharkMan (86, シャークマン in the Japanese games), 35 MB.

local cards = require("../rules/patch_cards/cards")

return define.patch_card {
    id = "sharkman",
    mb = 35,
    effects = {
        cards.hp(180),
        cards.body("aqua"),
        cards.attack(2),
        cards.rapid(2),
        cards.charge(2),
    },
}
