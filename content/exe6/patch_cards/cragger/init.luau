--!strict
-- The patch card Cragger (27, レムゴン in the Japanese games), 19 MB.

local cards = require("../rules/patch_cards/cards")

local card: PatchCard = {
    mb = 19,
    effects = {
        cards.hp(200),
        cards.undershirt(true),
        cards.bug(cards.hit_status("hp_bug")),
    },
}

return { cragger = card }
