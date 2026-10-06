--!strict
-- The patch card MetalMan (39, メタルマン in the Japanese games), 46 MB.

local cards = require("../rules/patch_cards/cards")
local variants = require("../objects/projectile/variants")

local card: PatchCard = {
    mb = 46,
    effects = {
        cards.hp(300),
        cards.super_armor(true),
        cards.attack_times(2),
        cards.charge_shot_kind(variants.breaking),
        cards.bug(cards.custom(-2)),
    },
}

return { metalman = card }
