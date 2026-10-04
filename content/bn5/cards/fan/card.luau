--!strict
-- The patch card Fan (58, ファンカー in the Japanese games), 15 MB.

local cards = require("../../rules/patch-cards/cards")
local chips = require("../../navis/megaman/weapons/chips")

return define.patch_card {
    id = "fan",
    mb = 15,
    effects = {
        cards.charged_shot(chips.tornado),
        cards.bug(cards.hp(-50)),
    },
}
