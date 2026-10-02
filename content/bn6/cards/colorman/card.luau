--!strict
-- The patch card ColorMan (85, カラードマン in the Japanese games), 32 MB.

local cards = require("../../rules/patch-cards/cards")
local variants = require("../../objects/projectile/variants")

return cards.card {
    id = "patch-card/colorman",
    name = "ColorMan",
    mb = 32,
    effects = {
        cards.hp(210),
        cards.charge_shot_kind(variants.charged_confusing),
        cards.bug(cards.back_special(nil)),
        cards.bug(cards.status_bug(1)),
    },
}
