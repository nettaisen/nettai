--!strict
-- The patch card WuNote (18, ウドノート in the Japanese games), 13 MB.

local cards = require("../../rules/patch-cards/cards")
local variants = require("../../objects/projectile/variants")

return cards.card {
    id = "patch-card/wunote",
    name = "WuNote",
    mb = 13,
    effects = {
        cards.hp(90),
        cards.body("wood"),
        cards.buster_shot(variants.grass),
        cards.hit_status("none"),
    },
}
