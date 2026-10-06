--!strict
-- The patch card DrillMan (89, ドリルマン in the Japanese games), 47 MB.

local cards = require("../rules/patch_cards/cards")
local chips = require("../navis/megaman/weapons/chips")

return define.patch_card {
    id = "drillman",
    mb = 47,
    effects = {
        cards.hp(200),
        cards.air_shoes(true),
        cards.charged_shot(chips.drilarm),
        cards.bug(cards.panel_trail("hole", 2)),
    },
}
