--!strict
-- The patch card MagicMan (85, マジックマン in the Japanese games), 44 MB.

local cards = require("../../rules/patch-cards/cards")

return define.patch_card {
    id = "magicman",
    mb = 44,
    effects = {
        cards.custom(1),
        cards.soul_time(2),
        cards.bug(cards.hp(-200)),
    },
}
