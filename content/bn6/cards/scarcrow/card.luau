--!strict
-- The patch card ScarCrow (25, カカジー in the Japanese games), 14 MB.

local cards = require("../../rules/patch-cards/cards")
local charge = require("./charge")

return define.patch_card {
    id = "scarcrow",
    mb = 14,
    effects = {
        cards.body("elec"),
        cards.charged_shot(charge),
        cards.chip_recovery(30),
        cards.bug(cards.buster_blanks(4, 2)),
    },
}
