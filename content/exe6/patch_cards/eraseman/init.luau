--!strict
-- The patch card EraseMan (49, キラーマン in the Japanese games), 40 MB.

local cards = require("../rules/patch_cards/cards")
local variants = require("../objects/projectile/variants")

local card: PatchCard = {
    mb = 40,
    effects = {
        cards.bug(cards.hp_percent(-10)),
        cards.mega_folder(1),
        cards.charge_shot_kind(variants.charged_hp_bug_marked),
        cards.bug(cards.hp_drain(1)),
    },
}

return { eraseman = card }
