--!strict
-- The patch card ErthDrgn (30, ツボリュウ in the Japanese games), 20 MB.

local cards = require("../rules/patch_cards/cards")
local chrgs = require("../lib/patch_cards/chrgs")

return define.patch_card {
    id = "erthdrgn",
    mb = 20,
    effects = {
        cards.hp(180),
        cards.bug(cards.charged_shot(chrgs)),
        cards.mega_folder(1),
        cards.bug(cards.status_bug(1)),
    },
}
