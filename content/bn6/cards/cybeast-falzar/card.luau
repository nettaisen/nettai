--!strict
-- The patch card Cybeast Falzar (116, 電脳獣ファルザー in the Japanese games), 70 MB.

local cards = require("../../rules/patch-cards")

return define.patch_card {
    id = "cybeast-falzar",
    name = "Cybeast Falzar",
    mb = 70,
    effects = {
        cards.hp_percent(40),
        cards.float_shoes(true),
        cards.air_shoes(true),
        cards.bug(cards.hp_drain(1)),
    },
}
