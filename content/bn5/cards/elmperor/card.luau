--!strict
-- The patch card Elmperor (76, エレンプラ in the Japanese games), 20 MB.

local cards = require("../../rules/patch-cards/cards")
local variants = require("../../objects/projectile/variants")

return define.patch_card {
    id = "elmperor",
    mb = 20,
    effects = {
        cards.charge_shot_kind(variants.charged_poisoning),
        cards.status_bug(0),
        cards.bug(cards.buster_shot(variants.plain)),
    },
}
