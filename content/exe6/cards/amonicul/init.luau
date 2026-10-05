--!strict
-- The patch card Amonicul (2, アモナキュール in the Japanese games), 16 MB.

local cards = require("../rules/patch-cards/cards")
local barriers = require("../lib/barriers")

return define.patch_card {
    id = "amonicul",
    mb = 16,
    effects = {
        cards.hp(80),
        cards.body("aqua"),
        cards.bug(cards.attack(-1)),
        cards.first_barrier(barriers.bubble),
    },
}
