--!strict
-- The patch card Zomon (26, ザエモン in the Japanese games), 20 MB.

local cards = require("../rules/patch_cards/cards")
local chips = require("../navis/megaman/weapons/chips")

local card: PatchCard = {
    mb = 20,
    effects = {
        cards.hp(60),
        cards.charged_shot(chips.katana),
        cards.bug(cards.buster_blanks(4, 2)),
    },
}

return { zomon = card }
