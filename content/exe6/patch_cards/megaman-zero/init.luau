--!strict
-- The patch card MegaMan-Zero (114, ロックマンゼロ in the Japanese games), 45 MB.

local cards = require("../rules/patch_cards/cards")
local b_sword = require("../lib/patch_cards/b_sword")
local charge = require("@self/charge")

return define.patch_card {
    id = "megaman-zero",
    mb = 45,
    effects = {
        cards.b_button(b_sword),
        cards.charged_shot(charge),
        cards.gauge("fast"),
    },
}
