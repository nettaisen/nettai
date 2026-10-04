--!strict
-- The patch card BubblMan (37, バブルマン in the Japanese games), 37 MB.

local cards = require("../../rules/patch-cards/cards")
local barriers = require("../../lib/barriers")

return define.patch_card {
    id = "bn5:bubblman",
    mb = 37,
    effects = {
        cards.body("aqua"),
        cards.first_barrier(barriers.bubble),
        cards.sneak_run(true),
        cards.bug(cards.hp(-50)),
    },
}
