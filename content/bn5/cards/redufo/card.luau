--!strict
-- The patch card RedUFO (64, UFOサニー in the Japanese games), 20 MB.

local cards = require("../../rules/patch-cards/cards")
local variants = require("../../objects/projectile/variants")

return define.patch_card {
    id = "redufo",
    mb = 20,
    effects = {
        cards.charge_shot_kind(variants.attack_90),
        cards.bug(cards.chip_recovery(0)),
        cards.bug(cards.step_bug(1)),
    },
}
