--!strict
-- The patch card Starfish (23, ヒトデスタ in the Japanese games), 15 MB.

local cards = require("../rules/patch_cards/cards")
local variants = require("../objects/projectile/variants")

local card: PatchCard = {
    mb = 15,
    effects = {
        cards.body("aqua"),
        cards.charge_shot_kind(variants.charged_bubbling),
    },
}

return { starfish = card }
