--!strict
-- The patch card DarkMech (83, アサシンメカ in the Japanese games), 22 MB.

local cards = require("../../rules/patch-cards")
local back_special = require("./back-special")

return define.patch_card {
    id = "darkmech",
    name = "DarkMech",
    mb = 22,
    effects = {
        cards.hp_percent(10),
        cards.back_special(back_special),
        cards.bug(cards.hp_drain(2)),
    },
}
