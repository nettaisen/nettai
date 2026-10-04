--!strict
-- The patch card Bass-Cross MegaMan (Team Colonel's) (111, フォルテクロス ロックマン in the Japanese games), 70 MB. Switched on,
-- MegaMan is in Hub Style (0x08137A58).

local cards = require("../rules/patch-cards/cards")
local buster = require("../navis/megaman/weapons/buster")
local hells_rolling = require("../navis/megaman/weapons/hells-rolling")

return define.patch_card {
    id = "bass-cross-megaman-colonel",
    mb = 70,
    effects = {
        cards.hp_percent(20),
        cards.float_shoes(true),
        cards.b_button(buster.tri_buster),
        cards.charged_shot(hells_rolling),
        cards.body("null"),
        cards.bug(cards.mega_folder(-2)),
        cards.hub_style(2),
    },
}
