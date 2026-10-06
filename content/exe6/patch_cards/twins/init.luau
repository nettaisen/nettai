--!strict
-- The patch card Twins (67, ツインズ in the Japanese games), 15 MB.

local cards = require("../rules/patch_cards/cards")
local variants = require("../objects/projectile/variants")

return define.patch_card {
    id = "twins",
    mb = 15,
    effects = {
        cards.hp(100),
        cards.attack(1),
        cards.bug(cards.charge_shot_kind(variants.charged)),
        cards.chip_recovery(20),
    },
}
