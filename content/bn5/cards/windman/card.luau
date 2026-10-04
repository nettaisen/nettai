--!strict
-- The patch card WindMan (92, ウインドマン in the Japanese games), 46 MB.

local cards = require("../../rules/patch-cards/cards")
local variants = require("../../objects/projectile/variants")

return define.patch_card {
    id = "bn5:windman",
    mb = 46,
    effects = {
        cards.hp(120),
        cards.air_shoes(true),
        cards.charge_shot_kind(variants.charged_attack_a0),
        cards.bug(cards.attack(-2)),
    },
}
