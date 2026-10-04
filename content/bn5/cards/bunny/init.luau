--!strict
-- The patch card Bunny (61, ラビリー in the Japanese games), 8 MB.

local cards = require("../rules/patch-cards/cards")
local zap_ring = require("../navis/megaman/weapons/zap-ring")

return define.patch_card {
    id = "bunny",
    mb = 8,
    effects = {
        cards.body("elec"),
        cards.charged_shot(zap_ring),
        cards.bug(cards.super_armor(false)),
        cards.bug(cards.status_guard(false)),
    },
}
