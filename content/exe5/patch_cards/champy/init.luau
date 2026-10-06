--!strict
-- The patch card Champy (28, チャンプル in the Japanese games), 15 MB.

local cards = require("../rules/patch_cards/cards")
local variants = require("../objects/projectile/variants")

local card: PatchCard = {
    mb = 15,
    effects = {
        cards.hp(100),
        cards.body("fire"),
        cards.rapid(1),
        cards.charge(1),
        cards.bug(cards.charge_shot_kind(variants.charged)),
    },
}

return { champy = card }
