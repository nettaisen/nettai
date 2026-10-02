--!strict
-- The patch card Punk (107, パンク in the Japanese games), 50 MB.

local cards = require("../../rules/patch-cards/cards")
local shield = require("../../navis/megaman/weapons/shield/weapon")
local variants = require("../../objects/projectile/variants")

return define.patch_card {
    id = "bn6:punk",
    mb = 50,
    effects = {
        cards.hp(400),
        cards.super_armor(true),
        cards.charge_shot_kind(variants.charged_confusing),
        cards.back_special(shield.reflect),
    },
}
