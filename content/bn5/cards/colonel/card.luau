--!strict
-- The patch card Colonel (94, カーネル in the Japanese games), 42 MB.

local cards = require("../../rules/patch-cards/cards")
local chips = require("../../navis/megaman/weapons/chips")

return define.patch_card {
    id = "bn5:colonel",
    mb = 42,
    effects = {
        cards.hp(280),
        cards.charged_shot(chips.tankcan),
    },
}
