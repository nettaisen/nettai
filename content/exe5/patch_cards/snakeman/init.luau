--!strict
-- The patch card SnakeMan (36, スネークマン in the Japanese games), 40 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 40,
    effects = {
        cards.hp_percent(25),
        cards.body("wood"),
        cards.millions(true),
    },
}

return { snakeman = card }
