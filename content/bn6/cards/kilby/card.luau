--!strict
-- The patch card Kilby (69, キルブー in the Japanese games), 24 MB.

local cards = require("../../rules/patch-cards")
local charge = require("./charge")

return define.patch_card {
    id = "kilby",
    name = "Kilby",
    mb = 24,
    effects = {
        cards.bug(cards.hp_percent(-20)),
        cards.body("wood"),
        cards.status_guard(true),
        cards.charged_shot(charge),
        cards.bug(cards.panel_trail("grass", 2)),
    },
}
