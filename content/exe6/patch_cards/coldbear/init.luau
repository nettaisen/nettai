--!strict
-- The patch card ColdBear (3, コルドベア in the Japanese games), 18 MB.

local cards = require("../rules/patch_cards/cards")
local charge = require("@self/charge")

local card: PatchCard = {
    mb = 18,
    effects = {
        cards.hp(140),
        cards.body("aqua"),
        cards.charged_shot(charge),
        cards.bug(cards.panel_trail("ice", 2)),
    },
}

return { coldbear = card }
