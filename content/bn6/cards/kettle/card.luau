--!strict
-- The patch card Kettle (29, ヤカーン in the Japanese games), 15 MB.

local cards = require("../../rules/patch-cards/cards")
local charge = require("./charge")
local shield = require("../../navis/megaman/weapons/shield/weapon")

return cards.card {
    id = "patch-card/kettle",
    name = "Kettle",
    mb = 15,
    effects = {
        cards.charged_shot(charge),
        cards.back_special(shield.shield),
        cards.chip_recovery(20),
        cards.bug(cards.custom_damage(40)),
        cards.hp_drain(0),
    },
}
