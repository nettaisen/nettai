--!strict
-- The patch card Powie (52, ポワルド in the Japanese games), 10 MB.

local cards = require("../rules/patch_cards/cards")
local variants = require("../objects/projectile/variants")

local card: PatchCard = {
    mb = 10,
    effects = {
        cards.hp(50),
        cards.charge_shot_kind(variants.charged_panel_breaking),
        cards.bug(cards.emotion_bug(1)),
    },
}

return { powie = card }
