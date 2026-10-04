--!strict
-- The patch card MettFire (6, メテファイア in the Japanese games), 20 MB.

local cards = require("../../rules/patch-cards/cards")
local b_button = require("./b-button")

return define.patch_card {
    id = "mettfire",
    mb = 20,
    effects = {
        cards.body("fire"),
        cards.b_button(b_button),
        cards.bug(cards.step_bug(1)),
    },
}
