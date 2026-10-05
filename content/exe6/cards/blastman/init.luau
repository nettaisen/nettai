--!strict
-- The patch card BlastMan (52, ブラストマン in the Japanese games), 28 MB.

local cards = require("../rules/patch-cards/cards")

return define.patch_card {
    id = "blastman",
    mb = 28,
    effects = {
        cards.bug(cards.hp(-80)),
        cards.body("fire"),
        cards.float_shoes(true),
        cards.attack(1),
        cards.panel_trail("none", 0),
    },
}
