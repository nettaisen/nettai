--!strict
-- The patch card Beetle (9, ボンビートル in the Japanese games), 11 MB.

local cards = require("../../rules/patch-cards/cards")
local b_button = require("./b-button")
local chrgs = require("../../lib/patch-cards/chrgs")

return cards.card {
    id = "patch-card/beetle",
    mb = 11,
    effects = {
        cards.hp(100),
        cards.b_button(b_button),
        cards.bug(cards.charged_shot(chrgs)),
    },
}
