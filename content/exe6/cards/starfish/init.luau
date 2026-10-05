--!strict
-- The patch card Starfish (23, ヒトデスタ in the Japanese games), 15 MB.

local cards = require("../rules/patch-cards/cards")
local variants = require("../objects/projectile/variants")

return define.patch_card {
    id = "starfish",
    mb = 15,
    effects = {
        cards.body("aqua"),
        cards.charge_shot_kind(variants.charged_bubbling),
    },
}
