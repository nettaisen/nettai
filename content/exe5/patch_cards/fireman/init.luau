--!strict
-- The patch card FireMan (33, ファイアマン in the Japanese games), 43 MB.

local cards = require("../rules/patch_cards/cards")
local fire_arm = require("../navis/megaman/weapons/fire_arm")

local card: PatchCard = {
    mb = 43,
    effects = {
        cards.hp_percent(30),
        cards.body("fire"),
        cards.bug(cards.buster_shot(nil)),
        cards.charged_shot(fire_arm),
    },
}

return { fireman = card }
