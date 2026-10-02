--!strict
-- The patch card Skarab (72, スカラビア in the Japanese games), 7 MB.

local cards = require("../../rules/patch-cards/cards")
local variants = require("../../objects/projectile/variants")

return cards.card {
    id = "patch-card/skarab",
    name = "Skarab",
    mb = 7,
    effects = {
        cards.bug(cards.hp(-70)),
        cards.charge_shot_kind(variants.charged_confusing),
        cards.custom_hp_drain(0),
    },
}
