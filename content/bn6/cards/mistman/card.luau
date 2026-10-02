--!strict
-- The patch card MistMan (93, ミストマン in the Japanese games), 37 MB.

local cards = require("../../rules/patch-cards")
local variants = require("../../objects/projectile/variants")

return define.patch_card {
    id = "mistman",
    name = "MistMan",
    mb = 37,
    effects = {
        cards.hp(260),
        cards.buster_shot(variants.poisoning),
        cards.bug(cards.chip_recovery(0)),
    },
}
