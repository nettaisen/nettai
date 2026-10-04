--!strict
-- The patch card CirKill (77, サーキラー in the Japanese games), 18 MB.

local cards = require("../../rules/patch-cards/cards")
local chips = require("../../navis/megaman/weapons/chips")

return define.patch_card {
    id = "bn5:cirkill",
    mb = 18,
    effects = {
        cards.charged_shot(chips.circgun),
        cards.bug(cards.step_bug(1)),
    },
}
