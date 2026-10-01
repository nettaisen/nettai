--!strict
-- What the ruleset needs from the test pack by role
-- (docs/design/content-model-v2.md §7.4): AntiDmg's counter, and the
-- charged shot a navi's request starts.

local ticker = require("../lib/ticker")

return define.roles {
    actions = {
        anti_damage_counter = ticker.action { id = "test/anti-damage-counter", length = 4 },
        forced_charged_shot = ticker.action { id = "test/forced-charged-shot", length = 3 },
    },
}
