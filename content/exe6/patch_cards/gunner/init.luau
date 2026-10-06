--!strict
-- The patch card Gunner (76, ガンナー in the Japanese games), 11 MB.

local cards = require("../rules/patch_cards/cards")
local charge = require("@self/charge")

return define.patch_card {
    id = "gunner",
    mb = 11,
    effects = {
        cards.hp(70),
        cards.bug(cards.undershirt(false)),
        cards.charged_shot(charge),
    },
}
