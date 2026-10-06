--!strict
-- The patch card Ghost (3, ゴースラー in the Japanese games), 15 MB.

local cards = require("../rules/patch_cards/cards")
local instants = require("../navis/megaman/weapons/instants")

local card: PatchCard = {
    mb = 15,
    effects = {
        cards.float_shoes(true),
        cards.charged_shot(instants.invisible),
        cards.bug(cards.hp(-80)),
    },
}

return { ghost = card }
