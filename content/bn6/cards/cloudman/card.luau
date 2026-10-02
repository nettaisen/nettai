--!strict
-- The patch card CloudMan (100, クラウドマン in the Japanese games), 40 MB.

local cards = require("../../rules/patch-cards/cards")
local charge = require("./charge")

return define.patch_card {
    id = "cloudman",
    mb = 40,
    effects = {
        cards.hp(230),
        cards.body("elec"),
        cards.charged_shot(charge),
        cards.bug(cards.mega_folder(-1)),
    },
}
