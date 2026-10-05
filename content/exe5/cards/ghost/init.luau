--!strict
-- The patch card Ghost (3, ゴースラー in the Japanese games), 15 MB.

local cards = require("../rules/patch-cards/cards")
local instants = require("../navis/megaman/weapons/instants")

return define.patch_card {
    id = "ghost",
    mb = 15,
    effects = {
        cards.float_shoes(true),
        cards.charged_shot(instants.invisible),
        cards.bug(cards.hp(-80)),
    },
}
