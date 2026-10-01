--!strict
-- What the ruleset needs from the engine's test content by role
-- (docs/design/content-model-v2.md §7.4): the BN6 overlay's modules the
-- test content registers, the v1 ones through their legacy markers. (The
-- test pack, crates/bn6-battle/testdata/pack, brings its own.)

local charged_shot = require("../navis/00-megaman/weapons/charged-shot/weapon")

return define.roles {
    actions = {
        forced_charged_shot = charged_shot.action,
        -- The strike the navi's request 0x80000 starts from idle (the
        -- machine swords' strike by the attack's variant).
        stun_strike = { legacy = { action = 0x49 } },
        -- What a Cross navi protected at the battle's end runs.
        cross_protect = { legacy = { action = 0x4D } },
        -- The turn L or R starts where turning is enabled.
        turn = { legacy = { action = 0x3B } },
        -- A Cross navi's knock-out.
        cross_death = { legacy = { action = 0x4C } },
        -- A Cross navi's volley.
        volley = { legacy = { action = 0x30 } },
        -- SlashCross's charged slash, which the Beast rush recognizes.
        charged_sword = { legacy = { action = 0x41 } },
        -- The Beast forms' charged claw.
        beast_claw = { legacy = { action = 0x52 } },
        -- DustCross Beast's scatter, during which the navi isn't grounded.
        dust_beast_scatter = { legacy = { action = 0x50 } },
    },
    kinds = {
        rock = { legacy = { kind = "rock" } },
        absorbed_obstacle = { legacy = { kind = "absorbed-obstacle" } },
        falling_rock = { legacy = { kind = "falling-rock" } },
    },
}
