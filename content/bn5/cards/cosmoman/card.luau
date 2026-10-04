--!strict
-- The patch card CosmoMan (45, コスモマン in the Japanese games), 44 MB.

local cards = require("../../rules/patch-cards/cards")

return define.patch_card {
    id = "cosmoman",
    mb = 44,
    effects = {
        cards.mega_folder(1),
        cards.giga_folder(1),
        cards.bug(cards.hp_percent(-20)),
    },
}
