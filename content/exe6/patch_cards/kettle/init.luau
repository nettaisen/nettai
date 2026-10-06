--!strict
-- The patch card Kettle (29, ヤカーン in the Japanese games), 15 MB.

local cards = require("../rules/patch_cards/cards")
local charge = require("@self/charge")
local shield = require("../navis/megaman/weapons/shield")

return define.patch_card {
    id = "kettle",
    mb = 15,
    effects = {
        cards.charged_shot(charge),
        cards.back_special(shield.shield),
        cards.chip_recovery(20),
        cards.bug(cards.custom_damage(40)),
        cards.hp_drain(0),
    },
}
