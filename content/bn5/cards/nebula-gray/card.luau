--!strict
-- The patch card Nebula-Gray (108, ネビュラグレイ in the Japanese games), 70 MB.

local cards = require("../../rules/patch-cards/cards")
local chips = require("../../navis/megaman/weapons/chips")

return define.patch_card {
    id = "bn5:nebula-gray",
    mb = 70,
    effects = {
        cards.charged_shot(chips.drksword),
        cards.bug(cards.hp_percent(-30)),
        cards.bug(cards.hp_drain(2)),
        cards.bug(cards.custom_hp_drain(2)),
    },
}
