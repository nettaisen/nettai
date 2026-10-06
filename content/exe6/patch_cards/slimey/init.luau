--!strict
-- The patch card Slimey (12, ジェライム in the Japanese games), 12 MB.

local cards = require("../rules/patch_cards/cards")
local charge = require("@self/charge")

return define.patch_card {
    id = "slimey",
    mb = 12,
    effects = {
        cards.hp(30),
        cards.body("aqua"),
        cards.charged_shot(charge),
        cards.bug(cards.gauge("normal")),
    },
}
