--!strict
-- The patch card Piranha (6, アーバルボーイ in the Japanese games), 10 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 10,
    effects = {
        cards.hp(100),
        cards.body("aqua"),
        cards.rapid(5),
        cards.bug(cards.tango(false)),
        cards.encounter_element("fish"),
    },
}

return { piranha = card }
