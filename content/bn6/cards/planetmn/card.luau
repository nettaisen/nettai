--!strict
-- The patch card PlanetMn (39, プラネットマン in the Japanese games), 40 MB.

local cards = require("../../rules/patch-cards")
local b_button = require("./b-button")

return define.patch_card {
    id = "planetmn",
    name = "PlanetMn",
    mb = 40,
    effects = {
        cards.hp(300),
        cards.body("wood"),
        cards.status_guard(true),
        cards.bug(cards.air_shoes(false)),
        cards.b_button(b_button),
    },
}
