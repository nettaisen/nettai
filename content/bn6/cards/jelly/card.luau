--!strict
-- The patch card Jelly (56, ジェリー in the Japanese games), 13 MB.

local cards = require("../../rules/patch-cards/cards")

return cards.card {
    id = "patch-card/jelly",
    name = "Jelly",
    mb = 13,
    effects = {
        cards.hp(120),
        cards.body("aqua"),
        cards.charge(4),
        cards.bug(cards.gauge("slow")),
    },
}
