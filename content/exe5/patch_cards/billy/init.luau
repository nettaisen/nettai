--!strict
-- The patch card Billy (8, ビリー in the Japanese games), 5 MB.

local cards = require("../rules/patch_cards/cards")
local chips = require("../navis/megaman/weapons/chips")

local card: PatchCard = {
    mb = 5,
    effects = {
        cards.body("elec"),
        cards.bug(cards.undershirt(false)),
        cards.charged_shot(chips.thunder),
    },
}

return { billy = card }
