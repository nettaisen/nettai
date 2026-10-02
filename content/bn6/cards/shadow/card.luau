--!strict
-- The patch card Shadow (8, ダークシャドー in the Japanese games), 6 MB.

local cards = require("../../rules/patch-cards")
local b_sword = require("../../lib/patch-cards/b-sword")
local invisible_charge = require("../../lib/patch-cards/invisible-charge")

return define.patch_card {
    id = "shadow",
    name = "Shadow",
    mb = 6,
    effects = {
        cards.bug(cards.status_guard(false)),
        cards.b_button(b_sword),
        cards.charged_shot(invisible_charge),
    },
}
