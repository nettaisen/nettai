--!strict
-- The patch card ShadoMan (34, シャドーマン in the Japanese games), 38 MB.

local cards = require("../../rules/patch-cards/cards")
local anti_damage = require("../../navis/megaman/weapons/anti-damage/weapon")
local charge = require("./charge")

return define.patch_card {
    id = "bn6:shadoman",
    mb = 38,
    effects = {
        cards.bug(cards.hp(-50)),
        cards.float_shoes(true),
        cards.charged_shot(charge),
        cards.back_special(anti_damage.weapon),
    },
}
