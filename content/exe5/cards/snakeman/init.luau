--!strict
-- The patch card SnakeMan (36, スネークマン in the Japanese games), 40 MB.

local cards = require("../rules/patch-cards/cards")

return define.patch_card {
    id = "snakeman",
    mb = 40,
    effects = {
        cards.hp_percent(25),
        cards.body("wood"),
        cards.millions(true),
    },
}
