--!strict
-- The patch card Quaker (21, クエイカー in the Japanese games), 15 MB.

local cards = require("../../rules/patch-cards/cards")

return define.patch_card {
    id = "bn5:quaker",
    mb = 15,
    effects = {
        cards.hp(150),
        cards.custom_hp_drain(0),
        cards.bug(cards.chip_recovery(0)),
    },
}
