--!strict
-- Made-up patch cards for the engine's tests of BN6's patch-cards system
-- (rules/patch-cards; BN6's cards are content/bn6/cards).

local cards = require("../rules/patch-cards/cards")
local barriers = require("../lib/barriers/barriers")
local variants = require("../objects/projectile/variants")

local test = {}

-- Stats: HP up 10%, then 30 more (its kinds' order, not the card's);
-- Attack +3; an Aqua body; a bug, the HP drain 2 levels up.
test.stats = define.patch_card {
    id = "test:test-stats",
    mb = 20,
    effects = {
        cards.hp(30),
        cards.hp_percent(10),
        cards.attack(3),
        cards.body("aqua"),
        cards.bug(cards.hp_drain(2)),
    },
}

-- Abilities and choices: SuperArmor; no FloatShoes; Barr200; the confusing
-- charged shot; Rush; the fast gauge; ChpShufl.
test.abilities = define.patch_card {
    id = "test:test-abilities",
    mb = 30,
    effects = {
        cards.super_armor(true),
        cards.bug(cards.float_shoes(false)),
        cards.first_barrier(barriers.barrier_200),
        cards.charge_shot_kind(variants.charged_confusing),
        cards.rush(true),
        cards.gauge("fast"),
        cards.chip_shuffle(true),
    },
}

-- A later card's write wins: Attack -1 from the first card's.
test.later = define.patch_card {
    id = "test:test-later",
    mb = 5,
    effects = { cards.attack(-1), cards.giga_folder(-2) },
}

return test
