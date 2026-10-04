--!strict
-- The patch card Moloko (24, マルモコ in the Japanese games), 5 MB.

local cards = require("../../rules/patch-cards/cards")
local barriers = require("../../lib/barriers")
local variants = require("../../objects/projectile/variants")

return define.patch_card {
    id = "bn5:moloko",
    mb = 5,
    effects = {
        cards.first_barrier(barriers.barrier_10),
        cards.bug(cards.charge_shot_kind(variants.charged)),
    },
}
