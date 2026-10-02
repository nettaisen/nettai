--!strict
-- The patch card BomBoy (17, ボムボーイ in the Japanese games), 14 MB.

local cards = require("../../rules/patch-cards/cards")
local variants = require("../../objects/projectile/variants")

return cards.card {
    id = "patch-card/bomboy",
    mb = 14,
    effects = {
        cards.buster_shot(variants.pushing),
        cards.bug(cards.charge_shot_kind(variants.charged)),
        cards.step_bug(0),
        cards.panel_trail("none", 0),
    },
}
