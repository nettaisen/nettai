--!strict
-- The patch card Puffy (55, プクール in the Japanese games), 12 MB.

local cards = require("../../rules/patch-cards")
local charge = require("./charge")

return define.patch_card {
    id = "puffball",
    name = "Puffy",
    mb = 12,
    effects = {
        cards.body("aqua"),
        cards.float_shoes(true),
        cards.charged_shot(charge),
        cards.bug(cards.back_special(nil)),
    },
}
