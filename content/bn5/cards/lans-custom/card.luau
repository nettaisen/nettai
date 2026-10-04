--!strict
-- The patch card Lan's Custom (96, 熱斗の カスタマイズ in the Japanese games), 25 MB.

local cards = require("../../rules/patch-cards/cards")
local buster = require("../../navis/megaman/weapons/buster")
local charged_shot = require("../../navis/megaman/weapons/charged-shot")

return define.patch_card {
    id = "bn5:lans-custom",
    mb = 25,
    effects = {
        cards.body("null"),
        cards.bug(cards.b_button(buster.weapon)),
        cards.bug(cards.charged_shot(charged_shot.program)),
        cards.mega_folder(2),
    },
}
