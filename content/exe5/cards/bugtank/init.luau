--!strict
-- The patch card Bugtank (51, カブタンク in the Japanese games), 7 MB.

local cards = require("../rules/patch-cards/cards")
local chips = require("../navis/megaman/weapons/chips")

return define.patch_card {
    id = "bugtank",
    mb = 7,
    effects = {
        cards.hp(80),
        cards.charged_shot(chips.minibomb),
        cards.bug(cards.custom_damage(50)),
    },
}
