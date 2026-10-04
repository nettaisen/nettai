--!strict
-- The patch card HeatMan (86, ヒートマン in the Japanese games), 45 MB.

local cards = require("../../rules/patch-cards/cards")
local charged_shot = require("../../navis/megaman/weapons/charged-shot")
local variants = require("../../objects/projectile/variants")

return define.patch_card {
    id = "heatman",
    mb = 45,
    effects = {
        cards.hp(100),
        cards.body("fire"),
        cards.attack(2),
        cards.charge_shot_kind(variants.charged_lava),
        cards.bug(cards.charged_shot(charged_shot.program)),
    },
}
