--!strict
-- The patch card VideoMan (43, ビデオマン in the Japanese games), 32 MB.

local cards = require("../../rules/patch-cards")
local variants = require("../../objects/projectile/variants")

return define.patch_card {
    id = "videoman",
    name = "VideoMan",
    mb = 32,
    effects = {
        cards.bug(cards.hp_percent(-10)),
        cards.charge_shot_kind(variants.charged_road),
        cards.gauge("fast"),
    },
}
