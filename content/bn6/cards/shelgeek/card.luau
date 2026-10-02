--!strict
-- The patch card ShelGeek (62, シェルキー in the Japanese games), 8 MB.

local cards = require("../../rules/patch-cards/cards")
local shield = require("../../navis/megaman/weapons/shield/weapon")

return define.patch_card {
    id = "shelgeek",
    mb = 8,
    effects = {
        cards.body("aqua"),
        cards.rapid(4),
        cards.back_special(shield.shield),
    },
}
