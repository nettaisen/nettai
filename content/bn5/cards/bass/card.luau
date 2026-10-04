--!strict
-- The patch card Bass (34, フォルテ in the Japanese games), 45 MB.

local cards = require("../../rules/patch-cards/cards")

return define.patch_card {
    id = "bass",
    mb = 45,
    effects = {
        cards.hp(200),
        cards.attack(5),
        cards.rapid(5),
        cards.charge(5),
        cards.bug(cards.soul_time(-2)),
    },
}
