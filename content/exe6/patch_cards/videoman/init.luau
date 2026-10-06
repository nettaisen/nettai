--!strict
-- The patch card VideoMan (43, ビデオマン in the Japanese games), 32 MB.

local cards = require("../rules/patch_cards/cards")
local variants = require("../objects/projectile/variants")

local card: PatchCard = {
    mb = 32,
    effects = {
        cards.bug(cards.hp_percent(-10)),
        cards.charge_shot_kind(variants.charged_road),
        cards.gauge("fast"),
    },
}

return { videoman = card }
