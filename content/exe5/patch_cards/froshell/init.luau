--!strict
-- The patch card Froshell (53, フロシェル in the Japanese games), 16 MB.

local cards = require("../rules/patch_cards/cards")
local chips = require("../navis/megaman/weapons/chips")

local card: PatchCard = {
    mb = 16,
    effects = {
        cards.hp(140),
        cards.b_button(chips.minibomb),
        cards.bug(cards.first_barrier(nil)),
    },
}

return { froshell = card }
