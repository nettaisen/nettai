--!strict
-- The patch card Elmperor (76, エレンプラ in the Japanese games), 20 MB.

local cards = require("../rules/patch_cards/cards")
local variants = require("../objects/projectile/variants")

local card: PatchCard = {
    mb = 20,
    effects = {
        cards.bug(cards.buster_shot(nil)),
        cards.charge_shot_kind(variants.charged_poisoning),
        cards.status_bug(0),
    },
}

return { elmperor = card }
