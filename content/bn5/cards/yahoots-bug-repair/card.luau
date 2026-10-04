--!strict
-- The patch card Yahoot's Bug Repair (99, マハ・ジャラマの バグしゅうせい in the Japanese games), 17 MB.

local cards = require("../../rules/patch-cards/cards")

return define.patch_card {
    id = "bn5:yahoots-bug-repair",
    mb = 17,
    effects = {
        cards.step_bug(0),
        cards.panel_trail("none", 0),
    },
}
