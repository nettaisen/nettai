--!strict
-- The patch card TopMan (96, タップマン in the Japanese games), 28 MB.

local cards = require("../../rules/patch-cards")
local chrgs = require("../../lib/patch-cards/chrgs")

return define.patch_card {
    id = "topman",
    name = "TopMan",
    mb = 28,
    effects = {
        cards.hp_percent(10),
        cards.bug(cards.charged_shot(chrgs)),
        cards.buster_blanks(0, 2),
        cards.bug(cards.hit_status("hp_bug")),
    },
}
