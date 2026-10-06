--!strict
-- The patch card Diodon (22, センボン in the Japanese games), 11 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 11,
    effects = {
        cards.body("aqua"),
        cards.rapid(4),
        cards.buster_blanks(0, 0),
    },
}

return { diodon = card }
