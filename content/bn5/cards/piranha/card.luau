--!strict
-- The patch card Piranha (6, アーバルボーイ in the Japanese games), 10 MB.

local cards = require("../../rules/patch-cards/cards")

return define.patch_card {
    id = "bn5:piranha",
    mb = 10,
    effects = {
        cards.hp(100),
        cards.body("aqua"),
        cards.rapid(5),
        cards.encounter_element("fish"),
        cards.bug(cards.tango(false)),
    },
}
