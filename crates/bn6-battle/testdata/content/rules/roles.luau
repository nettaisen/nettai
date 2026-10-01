--!strict
-- What the ruleset needs from the engine's test content by role
-- (docs/design/content-model-v2.md §7.4): the BN6 overlay's modules the
-- test content registers, the v1 ones through their legacy markers. (The
-- test pack, crates/bn6-battle/testdata/pack, brings its own.)

local absorbed_obstacle = require("../objects/absorbed-obstacle/absorbed_obstacle")
local barriers = require("../lib/barriers/barriers")
local beast_claw = require("../navis/00-megaman/weapons/beast-claw/weapon")
local charged_shot = require("../navis/00-megaman/weapons/charged-shot/weapon")
local dust_beast_scatter = require("../navis/00-megaman/forms/dustcross-beast/scatter")
local falling_rock = require("../objects/falling-rock/falling_rock")
local ground_drill = require("../navis/00-megaman/forms/groundcross/drill")
local slash_beast_lunge = require("../navis/00-megaman/forms/slashcross-beast/lunge")
local slash_charge = require("../navis/00-megaman/forms/slashcross/charge")
local supports = require("../lib/supports/controller")

return define.roles {
    actions = {
        forced_charged_shot = charged_shot.action,
        -- The strike the navi's request 0x80000 starts from idle (the
        -- machine swords' strike by the attack's variant).
        stun_strike = { legacy = { action = 0x49 } },
        -- What a Cross navi protected at the battle's end runs
        -- (GroundCross's drill).
        cross_protect = ground_drill.action,
        -- The turn L or R starts where turning is enabled.
        turn = { legacy = { action = 0x3B } },
        -- A Cross navi's knock-out (SlashCross Beast's lunge).
        cross_death = slash_beast_lunge.action,
        -- A Cross navi's volley.
        volley = { legacy = { action = 0x30 } },
        -- SlashCross's charged slash, which the Beast rush recognizes.
        charged_sword = slash_charge.action,
        -- The Beast forms' charged claw.
        beast_claw = beast_claw.action,
        -- DustCross Beast's scatter, during which the navi isn't grounded.
        dust_beast_scatter = dust_beast_scatter.action,
    },
    -- (AntiRecv's counterattack isn't in the test content: its role stays
    -- unfilled.)
    kinds = {
        absorbed_obstacle = absorbed_obstacle.kind,
        falling_rock = falling_rock.kind,
        support = supports.kind,
    },
    hooks = {
        -- The NaviCust FirstBarrier, as BN6's roles fill it: the Barrier
        -- chip's barrier and its visual.
        first_barrier = function(navi: Object)
            barriers.raise(navi, barriers.barrier_10, true)
        end,
    },
}
