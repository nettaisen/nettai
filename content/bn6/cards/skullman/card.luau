--!strict
-- The patch card SkullMan (33, スカルマン in the Japanese games), 30 MB.

local cards = require("../../rules/patch-cards/cards")
local variants = require("../../objects/projectile/variants")

return define.patch_card {
    id = "skullman",
    mb = 30,
    effects = {
        cards.hp(100),
        cards.charge_shot_kind(variants.charged_hp_bug),
        cards.bug(cards.hp_drain(1)),
        cards.bug(cards.status_bug(1)),
    },
}
