--!strict
-- The patch card KillrEye (20, キラーズアイ in the Japanese games), 21 MB.

local cards = require("../rules/patch-cards/cards")
local variants = require("../objects/projectile/variants")

return define.patch_card {
    id = "killreye",
    mb = 21,
    effects = {
        cards.body("elec"),
        cards.charge_shot_kind(variants.charged_paralyzing),
        cards.bug(cards.hp_drain(4)),
    },
}
