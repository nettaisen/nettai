--!strict
-- The patch card BassGS (105, フォルテGS in the Japanese games), 59 MB.

local cards = require("../../rules/patch-cards/cards")
local variants = require("../../objects/projectile/variants")

return define.patch_card {
    id = "bn5:bassgs",
    mb = 59,
    effects = {
        cards.hp(300),
        cards.charge_shot_kind(variants.charged_hp_bug),
        cards.bug(cards.status_bug(2)),
    },
}
