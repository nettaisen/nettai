--!strict
-- The patch card Django (112, ジャンゴ in the Japanese games), 52 MB.

local cards = require("../../rules/patch-cards/cards")
local charge = require("./charge")

return cards.card {
    id = "patch-card/django",
    mb = 52,
    effects = {
        cards.hp(200),
        cards.status_guard(true),
        cards.undershirt(true),
        cards.charged_shot(charge),
    },
}
