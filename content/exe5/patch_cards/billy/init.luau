--!strict
-- The patch card Billy (8, ビリー in the Japanese games), 5 MB.

local cards = require("../rules/patch_cards/cards")
local chips = require("../navis/megaman/weapons/chips")

return define.patch_card {
    id = "billy",
    mb = 5,
    effects = {
        cards.body("elec"),
        cards.charged_shot(chips.thunder),
        cards.bug(cards.undershirt(false)),
    },
}
