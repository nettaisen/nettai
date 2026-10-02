--!strict
-- The patch card Cragger (27, レムゴン in the Japanese games), 19 MB.

local cards = require("../../rules/patch-cards")

return define.patch_card {
    id = "cragger",
    name = "Cragger",
    mb = 19,
    effects = {
        cards.hp(200),
        cards.undershirt(true),
        cards.bug(cards.hit_status("hp_bug")),
    },
}
