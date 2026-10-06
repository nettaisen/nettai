--!strict
-- The patch card LarkMan (47, スワローマン in the Japanese games), 36 MB.

local cards = require("../rules/patch_cards/cards")
local variants = require("../objects/projectile/variants")

return define.patch_card {
    id = "larkman",
    mb = 36,
    effects = {
        cards.air_shoes(true),
        cards.charge_shot_kind(variants.chip_destruction),
        cards.beat(true),
        cards.bug(cards.hit_status("confuse")),
    },
}
