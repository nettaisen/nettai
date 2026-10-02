--!strict
-- The patch card Armadill (28, アルマン in the Japanese games), 12 MB.

local cards = require("../../rules/patch-cards")
local variants = require("../../objects/projectile/variants")

return define.patch_card {
    id = "armadill",
    name = "Armadill",
    mb = 12,
    effects = {
        cards.hp_percent(5),
        cards.bug(cards.charge(-3)),
        cards.charge_shot_kind(variants.breaking),
    },
}
