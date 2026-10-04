--!strict
-- The patch card Bass-Cross MegaMan (Team ProtoMan's) (111, フォルテクロス ロックマン in the Japanese games), 70 MB. Switched on,
-- MegaMan is in Hub Style (0x08137A58).

local cards = require("../rules/patch-cards/cards")
local buster = require("../navis/megaman/weapons/buster")
local rapid_buster = require("../navis/megaman/weapons/rapid-buster")

return define.patch_card {
    id = "bass-cross-megaman-protoman",
    mb = 70,
    effects = {
        cards.hp(200),
        cards.super_armor(true),
        cards.b_button(buster.tri_buster),
        cards.charged_shot(rapid_buster),
        cards.body("null"),
        cards.bug(cards.mega_folder(-2)),
        cards.hub_style(1),
    },
}
