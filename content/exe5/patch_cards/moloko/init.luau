--!strict
-- The patch card Moloko (24, マルモコ in the Japanese games), 5 MB.

local cards = require("../rules/patch_cards/cards")
local barriers = require("../lib/barriers")
local variants = require("../objects/projectile/variants")

local card: PatchCard = {
    mb = 5,
    effects = {
        cards.bug(cards.charge_shot_kind(variants.charged)),
        cards.first_barrier(barriers.barrier_10),
    },
}

return { moloko = card }
