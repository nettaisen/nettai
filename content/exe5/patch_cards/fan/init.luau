--!strict
-- The patch card Fan (58, ファンカー in the Japanese games), 15 MB.

local cards = require("../rules/patch_cards/cards")
local chips = require("../navis/megaman/weapons/chips")

local card: PatchCard = {
    mb = 15,
    effects = {
        cards.bug(cards.hp(-50)),
        cards.charged_shot(chips.tornado),
    },
}

return { fan = card }
