--!strict
-- The patch card Quaker (21, クエイカー in the Japanese games), 15 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 15,
    effects = {
        cards.hp(150),
        cards.bug(cards.chip_recovery(0)),
        cards.custom_hp_drain(0),
    },
}

return { quaker = card }
