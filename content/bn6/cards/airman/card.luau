--!strict
-- The patch card AirMan (88, エアーマン in the Japanese games), 34 MB.

local cards = require("../../rules/patch-cards/cards")
local charge = require("./charge")
local variants = require("../../objects/projectile/variants")

return define.patch_card {
    id = "airman",
    mb = 34,
    effects = {
        cards.hp(150),
        cards.buster_shot(variants.shoving),
        cards.charged_shot(charge),
        cards.bug(cards.first_barrier(nil)),
        cards.buster_blanks(0, 0),
    },
}
