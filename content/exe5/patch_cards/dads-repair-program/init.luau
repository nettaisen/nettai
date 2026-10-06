--!strict
-- The patch card Dad's Repair Program (109, パパのしゅうせい プログラム in the Japanese games), 50 MB.

local cards = require("../rules/patch_cards/cards")

return define.patch_card {
    id = "dads-repair-program",
    mb = 50,
    effects = {
        cards.bug_stopper(1),
    },
}
