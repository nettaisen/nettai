--!strict
-- The patch card Marina (74, マリーナ in the Japanese games), 14 MB.

local cards = require("../../rules/patch-cards/cards")
local barriers = require("../../lib/barriers/barriers")

return cards.card {
    id = "patch-card/marina",
    mb = 14,
    effects = {
        cards.hp(90),
        cards.body("aqua"),
        cards.bug(cards.super_armor(false)),
        cards.first_barrier(barriers.bubble),
    },
}
