--!strict
-- The patch card Poitton (57, ポイットン in the Japanese games), 13 MB.

local cards = require("../../rules/patch-cards/cards")
local barriers = require("../../lib/barriers/barriers")

return cards.card {
    id = "patch-card/poitton",
    mb = 13,
    effects = {
        cards.bug(cards.hp(-40)),
        cards.body("wood"),
        cards.undershirt(true),
        cards.first_barrier(barriers.barrier_10),
        cards.hit_status("none"),
    },
}
