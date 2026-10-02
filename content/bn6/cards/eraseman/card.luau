--!strict
-- The patch card EraseMan (49, キラーマン in the Japanese games), 40 MB.

local cards = require("../../rules/patch-cards/cards")
local variants = require("../../objects/projectile/variants")

return define.patch_card {
    id = "eraseman",
    mb = 40,
    effects = {
        cards.bug(cards.hp_percent(-10)),
        cards.charge_shot_kind(variants.charged_hp_bug_marked),
        cards.mega_folder(1),
        cards.bug(cards.hp_drain(1)),
    },
}
