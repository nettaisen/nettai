--!strict
-- The patch card Beetle (9, ボンビートル in the Japanese games), 11 MB.

local cards = require("../rules/patch_cards/cards")
local b_button = require("@self/b_button")
local chrgs = require("../lib/patch_cards/chrgs")

return define.patch_card {
    id = "beetle",
    mb = 11,
    effects = {
        cards.hp(100),
        cards.b_button(b_button),
        cards.bug(cards.charged_shot(chrgs)),
    },
}
