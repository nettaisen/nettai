--!strict
-- The patch card Lan's Custom (96, 熱斗の カスタマイズ in the Japanese games), 25 MB.

local cards = require("../rules/patch_cards/cards")
local buster = require("../navis/megaman/weapons/buster")
local charged_shot = require("../navis/megaman/weapons/charged_shot")

local card: PatchCard = {
    mb = 25,
    effects = {
        cards.body("null"),
        cards.mega_folder(2),
        cards.bug(cards.b_button(buster.weapon)),
        cards.bug(cards.charged_shot(charged_shot.program)),
    },
}

return { ["lans-custom"] = card }
