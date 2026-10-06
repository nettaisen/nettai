--!strict
-- The patch card BlastMan (52, ブラストマン in the Japanese games), 28 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 28,
    effects = {
        cards.bug(cards.hp(-80)),
        cards.body("fire"),
        cards.attack(1),
        cards.float_shoes(true),
        cards.panel_trail("none", 0),
    },
}

return { blastman = card }
