--!strict
-- The patch card MagicMan (85, マジックマン in the Japanese games), 44 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 44,
    effects = {
        cards.custom(1),
        cards.soul_time(2),
        cards.bug(cards.hp(-200)),
    },
}

return { magicman = card }
