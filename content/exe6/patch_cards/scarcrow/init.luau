--!strict
-- The patch card ScarCrow (25, カカジー in the Japanese games), 14 MB.

local cards = require("../rules/patch_cards/cards")
local charge = require("@self/charge")

local card: PatchCard = {
    mb = 14,
    effects = {
        cards.body("elec"),
        cards.charged_shot(charge),
        cards.chip_recovery(30),
        cards.bug(cards.buster_blanks(4, 2)),
    },
}

return { scarcrow = card }
