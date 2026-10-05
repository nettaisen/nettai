--!strict
-- The patch card Puffy (22, センボン in the Japanese games), 11 MB.

local cards = require("../rules/patch-cards/cards")

return define.patch_card {
    id = "puffy",
    mb = 11,
    effects = {
        cards.body("aqua"),
        cards.rapid(4),
        cards.buster_blanks(0, 0),
    },
}
