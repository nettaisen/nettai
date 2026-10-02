--!strict
-- The patch card Amonicul (2, アモナキュール in the Japanese games), 16 MB.

local cards = require("../../rules/patch-cards/cards")
local barriers = require("../../lib/barriers/barriers")

return cards.card {
    id = "patch-card/amonicul",
    name = "Amonicul",
    mb = 16,
    effects = {
        cards.hp(80),
        cards.body("aqua"),
        cards.bug(cards.attack(-1)),
        cards.first_barrier(barriers.bubble),
    },
}
