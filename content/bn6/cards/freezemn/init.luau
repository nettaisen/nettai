--!strict
-- The patch card FreezeMn (89, フリーズマン in the Japanese games), 30 MB.

local cards = require("../rules/patch-cards/cards")
local barriers = require("../lib/barriers")
local variants = require("../objects/projectile/variants")

return define.patch_card {
    id = "freezemn",
    mb = 30,
    effects = {
        cards.bug(cards.hp_percent(-5)),
        cards.body("aqua"),
        cards.buster_shot(variants.ice),
        cards.first_barrier(barriers.barrier_200),
    },
}
