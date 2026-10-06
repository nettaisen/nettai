--!strict
-- The patch card ShadeMan (40, シェードマン in the Japanese games), 43 MB.

local cards = require("../rules/patch_cards/cards")
local chips = require("../navis/megaman/weapons/chips")

local card: PatchCard = {
    mb = 43,
    effects = {
        cards.bug(cards.hp(-70)),
        cards.bug(cards.soul_time(-1)),
        cards.float_shoes(true),
        cards.air_shoes(true),
        cards.charged_shot(chips.pulsar),
    },
}

return { shademan = card }
