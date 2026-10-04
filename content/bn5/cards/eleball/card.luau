--!strict
-- The patch card Eleball (73, パラボール in the Japanese games), 14 MB.

local cards = require("../../rules/patch-cards/cards")
local variants = require("../../objects/projectile/variants")

return define.patch_card {
    id = "bn5:eleball",
    mb = 14,
    effects = {
        cards.body("elec"),
        cards.float_shoes(true),
        cards.encounter_element("battery"),
        cards.bug(cards.buster_shot(variants.plain)),
    },
}
