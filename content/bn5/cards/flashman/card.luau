--!strict
-- The patch card FlashMan (88, フラッシュマン in the Japanese games), 46 MB.

local cards = require("../../rules/patch-cards/cards")
local variants = require("../../objects/projectile/variants")

return define.patch_card {
    id = "flashman",
    mb = 46,
    effects = {
        cards.hp_percent(20),
        cards.body("elec"),
        cards.charge_shot_kind(variants.charged_paralyzing),
        cards.bug(cards.back_special(nil)),
    },
}
