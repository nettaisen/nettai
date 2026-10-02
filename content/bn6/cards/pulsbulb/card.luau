--!strict
-- The patch card PulsBulb (77, パルフォロン in the Japanese games), 13 MB.

local cards = require("../../rules/patch-cards")
local charge = require("./charge")

return define.patch_card {
    id = "pulsbulb",
    name = "PulsBulb",
    mb = 13,
    effects = {
        cards.body("elec"),
        cards.charged_shot(charge),
        cards.bug(cards.emotion_bug(1)),
    },
}
