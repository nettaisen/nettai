--!strict
-- The patch card Froshell (53, フロシェル in the Japanese games), 16 MB.

local cards = require("../../rules/patch-cards/cards")
local chips = require("../../navis/megaman/weapons/chips")

return define.patch_card {
    id = "bn5:froshell",
    mb = 16,
    effects = {
        cards.hp(140),
        cards.b_button(chips.minibomb),
        cards.bug(cards.first_barrier(nil)),
    },
}
