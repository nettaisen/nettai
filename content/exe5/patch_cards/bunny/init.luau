--!strict
-- The patch card Bunny (61, ラビリー in the Japanese games), 8 MB.

local cards = require("../rules/patch_cards/cards")
local zap_ring = require("../navis/megaman/weapons/zap_ring")

local card: PatchCard = {
    mb = 8,
    effects = {
        cards.body("elec"),
        cards.bug(cards.super_armor(false)),
        cards.bug(cards.status_guard(false)),
        cards.charged_shot(zap_ring),
    },
}

return { bunny = card }
