--!strict
-- What the ruleset starts by role (docs/design/content-model-v2.md §7.4).
-- The trap chips' counters (AntiDmg's, AntiSwrd's, BodyGrd's) join when
-- those chips convert; until then their roles are unfilled.

local charged_shot = require("../navis/00-megaman/weapons/charged-shot/weapon")

return define.roles {
    actions = {
        -- The charged shot a navi's request starts from idle without its
        -- weapon's setup (the original's action 0x16).
        forced_charged_shot = charged_shot.action,
    },
}
