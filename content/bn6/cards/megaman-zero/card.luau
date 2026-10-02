--!strict
-- The patch card MegaMan-Zero (114, ロックマンゼロ in the Japanese games), 45 MB.

local cards = require("../../rules/patch-cards")
local b_sword = require("../../lib/patch-cards/b-sword")
local charge = require("./charge")

return define.patch_card {
    id = "megaman-zero",
    name = "MegaMan-Zero",
    mb = 45,
    effects = {
        cards.b_button(b_sword),
        cards.charged_shot(charge),
        cards.gauge("fast"),
    },
}
