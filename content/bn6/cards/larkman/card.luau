--!strict
-- The patch card LarkMan (47, スワローマン in the Japanese games), 36 MB.

local cards = require("../../rules/patch-cards/cards")
local variants = require("../../objects/projectile/variants")

return define.patch_card {
    id = "bn6:larkman",
    mb = 36,
    effects = {
        cards.air_shoes(true),
        cards.charge_shot_kind(variants.attack_90),
        cards.beat(true),
        cards.bug(cards.hit_status("confuse")),
    },
}
