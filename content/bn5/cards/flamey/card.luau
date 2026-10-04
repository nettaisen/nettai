--!strict
-- The patch card Flamey (62, フレイボー in the Japanese games), 20 MB.

local cards = require("../../rules/patch-cards/cards")
local chips = require("../../navis/megaman/weapons/chips")

return define.patch_card {
    id = "bn5:flamey",
    mb = 20,
    effects = {
        cards.hp(160),
        cards.body("fire"),
        cards.b_button(chips.crakout),
        cards.bug(cards.panel_trail("hole", 4)),
    },
}
