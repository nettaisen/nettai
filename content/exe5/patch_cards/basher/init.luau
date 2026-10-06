--!strict
-- The patch card Basher (68, ドゴーン in the Japanese games), 20 MB.

local cards = require("../rules/patch_cards/cards")
local variants = require("../objects/projectile/variants")

local card: PatchCard = {
    mb = 20,
    effects = {
        cards.body("fire"),
        cards.attack(1),
        cards.bug(cards.rapid(-3)),
        cards.charge_shot_kind(variants.charged_panel_breaking),
    },
}

return { basher = card }
