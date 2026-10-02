--!strict
-- The patch card ToadMan (37, トードマン in the Japanese games), 34 MB.

local cards = require("../../rules/patch-cards/cards")
local variants = require("../../objects/projectile/variants")

return cards.card {
    id = "patch-card/toadman",
    name = "ToadMan",
    mb = 34,
    effects = {
        cards.body("aqua"),
        cards.bug(cards.attack(-1)),
        cards.bug(cards.charge(-3)),
        cards.charge_shot_kind(variants.charged_paralyzing),
        cards.buster_blanks(0, 0),
    },
}
