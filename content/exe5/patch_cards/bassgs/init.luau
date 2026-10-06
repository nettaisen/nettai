--!strict
-- The patch card BassGS (105, フォルテGS in the Japanese games), 59 MB.

local cards = require("../rules/patch_cards/cards")
local variants = require("../objects/projectile/variants")

local card: PatchCard = {
    mb = 59,
    effects = {
        cards.hp(300),
        cards.charge_shot_kind(variants.charged_hp_bug),
        cards.bug(cards.status_bug(2)),
    },
}

return { bassgs = card }
