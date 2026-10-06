--!strict
-- The patch card ShadeMan (40, シェードマン in the Japanese games), 43 MB.

local cards = require("../rules/patch_cards/cards")
local chips = require("../navis/megaman/weapons/chips")

return define.patch_card {
    id = "shademan",
    mb = 43,
    effects = {
        cards.air_shoes(true),
        cards.float_shoes(true),
        cards.charged_shot(chips.pulsar),
        cards.bug(cards.hp(-70)),
        cards.bug(cards.soul_time(-1)),
    },
}
