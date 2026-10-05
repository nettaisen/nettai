--!strict
-- The patch card CanGuard (71, キャノガード in the Japanese games), 10 MB.

local cards = require("../rules/patch-cards/cards")
local charge = require("@self/charge")
local shield = require("../navis/megaman/weapons/shield")

return define.patch_card {
    id = "canguard",
    mb = 10,
    effects = {
        cards.bug(cards.float_shoes(false)),
        cards.charged_shot(charge),
        cards.back_special(shield.shield),
    },
}
