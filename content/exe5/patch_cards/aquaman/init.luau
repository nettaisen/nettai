--!strict
-- The patch card AquaMan (91, アクアマン in the Japanese games), 40 MB.

local cards = require("../rules/patch_cards/cards")
local chips = require("../navis/megaman/weapons/chips")

local card: PatchCard = {
    mb = 40,
    effects = {
        cards.hp(170),
        cards.body("aqua"),
        cards.bug(cards.undershirt(false)),
        cards.charged_shot(chips.seaseed),
    },
}

return { aquaman = card }
