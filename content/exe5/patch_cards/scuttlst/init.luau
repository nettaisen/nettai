--!strict
-- The patch card Scuttlst (18, ドリームビット in the Japanese games), 20 MB.

local cards = require("../rules/patch_cards/cards")
local barriers = require("../lib/barriers")

local card: PatchCard = {
    mb = 20,
    effects = {
        cards.first_barrier(barriers.barrier_100),
        cards.bug(cards.custom(-1)),
        cards.bug(cards.custom_damage(200)),
    },
}

return { scuttlst = card }
