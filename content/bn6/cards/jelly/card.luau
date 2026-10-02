--!strict
-- The patch card Jelly (56, ジェリー in the Japanese games), 13 MB.

local cards = require("../../rules/patch-cards/cards")

return define.patch_card {
    id = "jelly",
    mb = 13,
    effects = {
        cards.hp(120),
        cards.body("aqua"),
        cards.charge(4),
        cards.bug(cards.gauge("slow")),
    },
}
