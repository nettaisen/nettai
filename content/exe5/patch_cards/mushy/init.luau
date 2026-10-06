--!strict
-- The patch card Mushy (11, チャマッシュ in the Japanese games), 15 MB.

local cards = require("../rules/patch_cards/cards")
local variants = require("../objects/projectile/variants")

local card: PatchCard = {
    mb = 15,
    effects = {
        cards.body("wood"),
        cards.charge_shot_kind(variants.charged_confusing),
        cards.bug(cards.step_bug(1)),
    },
}

return { mushy = card }
