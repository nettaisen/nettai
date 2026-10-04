--!strict
-- The patch card AquaMan (91, アクアマン in the Japanese games), 40 MB.

local cards = require("../../rules/patch-cards/cards")
local chips = require("../../navis/megaman/weapons/chips")

return define.patch_card {
    id = "aquaman",
    mb = 40,
    effects = {
        cards.hp(170),
        cards.body("aqua"),
        cards.charged_shot(chips.seaseed),
        cards.bug(cards.undershirt(false)),
    },
}
