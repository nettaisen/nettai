--!strict
-- The patch card BubblMan (37, バブルマン in the Japanese games), 37 MB.

local cards = require("../rules/patch_cards/cards")
local barriers = require("../lib/barriers")

local card: PatchCard = {
    mb = 37,
    effects = {
        cards.bug(cards.hp(-50)),
        cards.body("aqua"),
        cards.first_barrier(barriers.bubble),
        cards.sneak_run(true),
    },
}

return { bubblman = card }
