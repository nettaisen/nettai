--!strict
-- The patch card LaserMan (93, レーザーマン in the Japanese games), 47 MB.

local cards = require("../rules/patch_cards/cards")
local variants = require("../objects/projectile/variants")

return define.patch_card {
    id = "laserman",
    mb = 47,
    effects = {
        cards.hp(250),
        cards.charge_shot_kind(variants.charged_uninstalling),
        cards.bug(cards.soul_time(-1)),
    },
}
