--!strict
-- The patch card Drixol (78, ドリクロール in the Japanese games), 17 MB.

local cards = require("../rules/patch_cards/cards")
local chips = require("../navis/megaman/weapons/chips")

local card: PatchCard = {
    mb = 17,
    effects = {
        cards.rapid(3),
        cards.charge(3),
        cards.charged_shot(chips.drilarm),
        cards.bug(cards.hit_status("confuse")),
    },
}

return { drixol = card }
