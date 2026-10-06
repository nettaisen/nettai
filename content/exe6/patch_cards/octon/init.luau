--!strict
-- The patch card Octon (61, タコバル in the Japanese games), 14 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 14,
    effects = {
        cards.hp(50),
        cards.body("aqua"),
        cards.attack(3),
        cards.bug(cards.buster_blanks(4, 1)),
    },
}

return { octon = card }
