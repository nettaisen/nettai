--!strict
-- The patch card DarkMan (95, ダークマン in the Japanese games), 25 MB.

local cards = require("../../rules/patch-cards/cards")
local charge = require("./charge")

return define.patch_card {
    id = "darkman",
    mb = 25,
    effects = {
        cards.charged_shot(charge),
        cards.bug(cards.step_bug(1)),
        cards.bug(cards.custom_damage(50)),
    },
}
