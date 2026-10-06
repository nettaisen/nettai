--!strict
-- The patch card DarkMan (95, ダークマン in the Japanese games), 25 MB.

local cards = require("../rules/patch_cards/cards")
local charge = require("@self/charge")

local card: PatchCard = {
    mb = 25,
    effects = {
        cards.charged_shot(charge),
        cards.bug(cards.step_bug(1)),
        cards.bug(cards.custom_damage(50)),
    },
}

return { darkman = card }
