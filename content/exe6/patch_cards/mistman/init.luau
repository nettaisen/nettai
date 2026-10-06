--!strict
-- The patch card MistMan (93, ミストマン in the Japanese games), 37 MB.

local cards = require("../rules/patch_cards/cards")
local variants = require("../objects/projectile/variants")

local card: PatchCard = {
    mb = 37,
    effects = {
        cards.hp(260),
        cards.buster_shot(variants.poisoning),
        cards.bug(cards.chip_recovery(0)),
    },
}

return { mistman = card }
