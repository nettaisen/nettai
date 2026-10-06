--!strict
-- The patch card OldStov (21, ダルスト in the Japanese games), 12 MB.

local cards = require("../rules/patch_cards/cards")
local charge = require("@self/charge")

local card: PatchCard = {
    mb = 12,
    effects = {
        cards.hp(130),
        cards.body("fire"),
        cards.charged_shot(charge),
        cards.bug(cards.hp_drain(1)),
    },
}

return { oldstov = card }
