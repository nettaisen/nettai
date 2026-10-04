--!strict
-- The patch card Roll (31, ロール in the Japanese games), 40 MB.

local cards = require("../../rules/patch-cards/cards")
local barriers = require("../../lib/barriers")
local variants = require("../../objects/projectile/variants")

return define.patch_card {
    id = "bn5:roll",
    mb = 40,
    effects = {
        cards.charge_shot_kind(variants.attack_90),
        cards.first_barrier(barriers.barrier_10),
        cards.hp_drain(0),
        cards.bug(cards.hp_percent(-5)),
    },
}
