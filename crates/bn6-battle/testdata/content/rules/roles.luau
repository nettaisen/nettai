--!strict
-- What the ruleset needs from the engine's test content by role
-- (docs/design/content-model-v2.md §7.4): the BN6 overlay's modules the
-- test content registers, the v1 ones through their legacy markers. (The
-- test pack, crates/bn6-battle/testdata/pack, brings its own.)

local charged_shot = require("../navis/00-megaman/weapons/charged-shot/weapon")

return define.roles {
    actions = {
        forced_charged_shot = charged_shot.action,
    },
    kinds = {
        rock = { legacy = { kind = "rock" } },
        absorbed_obstacle = { legacy = { kind = "absorbed-obstacle" } },
        falling_rock = { legacy = { kind = "falling-rock" } },
    },
}
