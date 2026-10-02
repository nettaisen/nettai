--!strict
-- The patch card NapalmMn (91, ナパームマン in the Japanese games), 39 MB.

local cards = require("../../rules/patch-cards/cards")
local charge = require("./charge")

return define.patch_card {
    id = "napalmmn",
    mb = 39,
    effects = {
        cards.body("fire"),
        cards.attack(1),
        cards.rapid(4),
        cards.charged_shot(charge),
        cards.bug(cards.custom_hp_drain(1)),
    },
}
