--!strict
-- The patch card YamatoMn (42, ヤマトマン in the Japanese games), 35 MB.

local cards = require("../rules/patch_cards/cards")
local shield = require("../navis/megaman/weapons/shield")

return define.patch_card {
    id = "yamatomn",
    mb = 35,
    effects = {
        cards.hp_percent(10),
        cards.attack(1),
        cards.back_special(shield.reflect),
        cards.bug(cards.custom_hp_drain(2)),
    },
}
