--!strict
-- The patch card SlashMan (48, スラッシュマン in the Japanese games), 31 MB.

local cards = require("../../rules/patch-cards/cards")
local charge = require("./charge")

return cards.card {
    id = "patch-card/slashman",
    name = "SlashMan",
    mb = 31,
    effects = {
        cards.attack(4),
        cards.charged_shot(charge),
        cards.bug(cards.buster_blanks(4, 1)),
    },
}
