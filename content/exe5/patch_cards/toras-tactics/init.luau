--!strict
-- The patch card Tora's Tactics (100, トラキチの 戦じゅつ in the Japanese games), 64 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 64,
    effects = {
        cards.rush(true),
        cards.beat(true),
        cards.tango(true),
    },
}

return { ["toras-tactics"] = card }
