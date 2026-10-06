--!strict
-- The patch card HeatMan (86, ヒートマン in the Japanese games), 45 MB.

local cards = require("../rules/patch_cards/cards")
local charged_shot = require("../navis/megaman/weapons/charged_shot")
local variants = require("../objects/projectile/variants")

local card: PatchCard = {
    mb = 45,
    effects = {
        cards.hp(100),
        cards.body("fire"),
        cards.attack(2),
        cards.bug(cards.charged_shot(charged_shot.program)),
        cards.charge_shot_kind(variants.charged_lava),
    },
}

return { heatman = card }
