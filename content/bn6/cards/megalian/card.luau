--!strict
-- The patch card Megalian (5, メガリア in the Japanese games), 20 MB.

local cards = require("../../rules/patch-cards/cards")
local barriers = require("../../lib/barriers/barriers")

return define.patch_card {
    id = "megalian",
    mb = 20,
    effects = {
        cards.bug(cards.hp(-80)),
        cards.air_shoes(true),
        cards.charge(4),
        cards.first_barrier(barriers.barrier_100),
    },
}
