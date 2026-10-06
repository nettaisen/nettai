--!strict
-- The patch card Totem (70, トトポール in the Japanese games), 13 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 13,
    effects = {
        cards.body("fire"),
        cards.chip_recovery(40),
        cards.mega_folder(1),
        cards.bug(cards.custom_hp_drain(1)),
    },
}

return { totem = card }
