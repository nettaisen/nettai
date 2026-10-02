--!strict
-- The patch card MagTect (15, マグテクト in the Japanese games), 15 MB.

local cards = require("../../rules/patch-cards/cards")
local variants = require("../../objects/projectile/variants")

return cards.card {
    id = "patch-card/magtect",
    mb = 15,
    effects = {
        cards.bug(cards.hp_percent(-5)),
        cards.body("elec"),
        cards.float_shoes(true),
        cards.charge_shot_kind(variants.charged_pulling),
    },
}
