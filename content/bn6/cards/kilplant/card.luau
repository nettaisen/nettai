--!strict
-- The patch card KilPlant (7, キルプラント in the Japanese games), 19 MB.

local cards = require("../../rules/patch-cards/cards")
local charge = require("./charge")

return define.patch_card {
    id = "kilplant",
    mb = 19,
    effects = {
        cards.body("wood"),
        cards.bug(cards.air_shoes(false)),
        cards.charged_shot(charge),
    },
}
