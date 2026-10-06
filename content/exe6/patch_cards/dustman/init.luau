--!strict
-- The patch card DustMan (51, ダストマン in the Japanese games), 37 MB.

local cards = require("../rules/patch_cards/cards")
local back_special = require("@self/back_special")
local variants = require("../objects/projectile/variants")

return define.patch_card {
    id = "dustman",
    mb = 37,
    effects = {
        cards.hp(140),
        cards.buster_shot(variants.cracking),
        cards.back_special(back_special),
        cards.bug(cards.panel_trail("crack", 1)),
    },
}
