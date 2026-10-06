--!strict
-- The patch card Django (112, ジャンゴ in the Japanese games), 52 MB.

local cards = require("../rules/patch_cards/cards")
local charge = require("@self/charge")

local card: PatchCard = {
    mb = 52,
    effects = {
        cards.hp(200),
        cards.status_guard(true),
        cards.undershirt(true),
        cards.charged_shot(charge),
    },
}

return { django = card }
