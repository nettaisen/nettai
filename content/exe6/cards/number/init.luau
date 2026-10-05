--!strict
-- The patch card Number (14, ナンバーズ in the Japanese games), 17 MB.

local cards = require("../rules/patch-cards/cards")
local chrgs = require("../lib/patch-cards/chrgs")

return define.patch_card {
    id = "number",
    mb = 17,
    effects = {
        cards.bug(cards.charged_shot(chrgs)),
        cards.custom(1),
        cards.mega_folder(1),
        cards.giga_folder(1),
        cards.bug(cards.custom_hp_drain(2)),
    },
}
