--!strict
-- The patch card Protecto (67, プロテクト in the Japanese games), 23 MB.

local cards = require("../../rules/patch-cards/cards")
local charged_shot = require("../../navis/megaman/weapons/charged-shot")

return define.patch_card {
    id = "protecto",
    mb = 23,
    effects = {
        cards.giga_folder(1),
        cards.bug(cards.undershirt(false)),
        cards.bug(cards.charged_shot(charged_shot.program)),
        cards.bug(cards.back_special(nil)),
    },
}
