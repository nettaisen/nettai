--!strict
-- The patch card BassXX (107, フォルテXX in the Japanese games), 70 MB.

local cards = require("../rules/patch_cards/cards")
local barriers = require("../lib/barriers")

local card: PatchCard = {
    mb = 70,
    effects = {
        cards.hp(400),
        cards.float_shoes(true),
        cards.air_shoes(true),
        cards.first_barrier(barriers.barrier_200),
        cards.bug(cards.hp_drain(4)),
    },
}

return { bassxx = card }
