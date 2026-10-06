--!strict
-- The patch card Roll (31, ロール in the Japanese games), 40 MB.

local cards = require("../rules/patch_cards/cards")
local barriers = require("../lib/barriers")
local variants = require("../objects/projectile/variants")

local card: PatchCard = {
    mb = 40,
    effects = {
        cards.bug(cards.hp_percent(-5)),
        cards.charge_shot_kind(variants.chip_destruction),
        cards.first_barrier(barriers.barrier_10),
        cards.hp_drain(0),
    },
}

return { roll = card }
