--!strict
-- What the ruleset needs from the test pack by role
-- (docs/design/content-model-v2.md §7.4): AntiDmg's counter, the
-- charged shot a navi's request starts, and what an encased obstacle
-- becomes (the BN6 content's).

local encased_bubble = require("../../objects/encased-bubble/bubble")
local ticker = require("../lib/ticker")

return define.roles {
    actions = {
        anti_damage_counter = ticker.action { id = "test/anti-damage-counter", length = 4 },
        forced_charged_shot = ticker.action { id = "test/forced-charged-shot", length = 3 },
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
    -- The engine's test content's kinds, which this pack's content comes
    -- with: named by their keys, since this pack's modules can't require
    -- that content's.
    kinds = {
        absorbed_obstacle = { legacy = { kind = "absorbed-obstacle" } },
        falling_rock = { legacy = { kind = "falling-rock" } },
    },
    chips = {
        -- What a zeroed chip field reads: the test content's chip 0 (a
        -- Rust-built record, which no module holds: by its key).
        zeroed = { legacy = { chip = "v1/chip-000" } },
    },
    hooks = {
        encased = encased_bubble.replace,
    },
}
