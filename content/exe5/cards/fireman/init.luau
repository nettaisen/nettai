--!strict
-- The patch card FireMan (33, ファイアマン in the Japanese games), 43 MB.

local cards = require("../rules/patch-cards/cards")
local fire_arm = require("../navis/megaman/weapons/fire-arm")

return define.patch_card {
    id = "fireman",
    mb = 43,
    effects = {
        cards.hp_percent(30),
        cards.body("fire"),
        cards.charged_shot(fire_arm),
        cards.bug(cards.buster_shot(nil)),
    },
}
