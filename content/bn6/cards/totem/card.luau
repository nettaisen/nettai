--!strict
-- The patch card Totem (70, トトポール in the Japanese games), 13 MB.

local cards = require("../../rules/patch-cards")

return define.patch_card {
    id = "totem",
    name = "Totem",
    mb = 13,
    effects = {
        cards.body("fire"),
        cards.chip_recovery(40),
        cards.mega_folder(1),
        cards.bug(cards.custom_hp_drain(1)),
    },
}
