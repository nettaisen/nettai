--!strict
-- The patch card WoodMan (81, ウッドマン in the Japanese games), 45 MB.

local cards = require("../../rules/patch-cards/cards")
local charged_shot = require("../../navis/megaman/weapons/charged-shot")
local variants = require("../../objects/projectile/variants")

return define.patch_card {
    id = "bn5:woodman",
    mb = 45,
    effects = {
        cards.hp_percent(25),
        cards.body("wood"),
        cards.status_guard(true),
        cards.bug(cards.buster_shot(variants.plain)),
        cards.bug(cards.charged_shot(charged_shot.program)),
    },
}
