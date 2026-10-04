--!strict
-- The patch card Basher (68, ドゴーン in the Japanese games), 20 MB.

local cards = require("../../rules/patch-cards/cards")
local variants = require("../../objects/projectile/variants")

return define.patch_card {
    id = "bn5:basher",
    mb = 20,
    effects = {
        cards.body("fire"),
        cards.attack(1),
        cards.charge_shot_kind(variants.charged_panel_breaking),
        cards.bug(cards.rapid(-3)),
    },
}
