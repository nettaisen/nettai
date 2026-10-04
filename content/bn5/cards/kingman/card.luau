--!strict
-- The patch card KingMan (90, キングマン in the Japanese games), 45 MB.

local cards = require("../../rules/patch-cards/cards")

return define.patch_card {
    id = "kingman",
    mb = 45,
    effects = {
        cards.custom(2),
        cards.mega_folder(1),
        cards.bug(cards.hp(-150)),
    },
}
