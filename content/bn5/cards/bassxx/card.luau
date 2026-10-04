--!strict
-- The patch card BassXX (107, フォルテXX in the Japanese games), 70 MB.

local cards = require("../../rules/patch-cards/cards")
local barriers = require("../../lib/barriers")

return define.patch_card {
    id = "bn5:bassxx",
    mb = 70,
    effects = {
        cards.hp(400),
        cards.float_shoes(true),
        cards.air_shoes(true),
        cards.first_barrier(barriers.barrier_200),
        cards.bug(cards.hp_drain(4)),
    },
}
