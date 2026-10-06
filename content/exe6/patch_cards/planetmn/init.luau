--!strict
-- The patch card PlanetMn (39, プラネットマン in the Japanese games), 40 MB.

local cards = require("../rules/patch_cards/cards")
local b_button = require("@self/b_button")

local card: PatchCard = {
    mb = 40,
    effects = {
        cards.hp(300),
        cards.body("wood"),
        cards.status_guard(true),
        cards.bug(cards.air_shoes(false)),
        cards.b_button(b_button),
    },
}

return { planetmn = card }
