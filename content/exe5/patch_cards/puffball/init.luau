--!strict
-- The patch card Puffball (13, バジリコ in the Japanese games), 16 MB.

local cards = require("../rules/patch_cards/cards")
local guards = require("../navis/megaman/weapons/guards")

local card: PatchCard = {
    mb = 16,
    effects = {
        cards.body("wood"),
        cards.charged_shot(guards.card_shield),
        cards.bug(cards.hp_drain(4)),
    },
}

return { puffball = card }
