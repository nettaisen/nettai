--!strict
-- The patch card BombCorn (78, ボムコーン in the Japanese games), 16 MB.

local cards = require("../../rules/patch-cards/cards")
local charge = require("./charge")

return define.patch_card {
    id = "bn6:bombcorn",
    mb = 16,
    effects = {
        cards.hp(110),
        cards.body("wood"),
        cards.charged_shot(charge),
        cards.bug(cards.hit_status("confuse")),
    },
}
