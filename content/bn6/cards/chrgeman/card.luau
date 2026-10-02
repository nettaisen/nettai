--!strict
-- The patch card ChrgeMan (102, チャージマン in the Japanese games), 31 MB.

local cards = require("../../rules/patch-cards")
local chrgs = require("../../lib/patch-cards/chrgs")

return define.patch_card {
    id = "chrgeman",
    name = "ChrgeMan",
    mb = 31,
    effects = {
        cards.attack(3),
        cards.charge(4),
        cards.bug(cards.charged_shot(chrgs)),
        cards.panel_trail("none", 0),
    },
}
