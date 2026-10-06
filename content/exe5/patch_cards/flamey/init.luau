--!strict
-- The patch card Flamey (62, フレイボー in the Japanese games), 20 MB.

local cards = require("../rules/patch_cards/cards")
local chips = require("../navis/megaman/weapons/chips")

local card: PatchCard = {
    mb = 20,
    effects = {
        cards.hp(160),
        cards.body("fire"),
        cards.b_button(chips.crakout),
        cards.bug(cards.panel_trail("hole", 4)),
    },
}

return { flamey = card }
