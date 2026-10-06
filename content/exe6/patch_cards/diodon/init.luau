--!strict
-- The patch card Diodon (22, センボン in the Japanese games), 11 MB.

local cards = require("../rules/patch_cards/cards")

return define.patch_card {
    id = "diodon",
    mb = 11,
    effects = {
        cards.body("aqua"),
        cards.rapid(4),
        cards.buster_blanks(0, 0),
    },
}
