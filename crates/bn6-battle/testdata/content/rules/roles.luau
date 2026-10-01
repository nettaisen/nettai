--!strict
-- What the ruleset needs from the engine's test content by role
-- (docs/design/content-model-v2.md §7.4): the BN6 overlay's modules the
-- test content registers, the v1 ones through their legacy markers. (The
-- test pack, crates/bn6-battle/testdata/pack, brings its own.)

local absorbed_obstacle = require("../objects/absorbed-obstacle/absorbed_obstacle")
local barriers = require("../lib/barriers/barriers")
local beast_claw = require("../navis/megaman/weapons/beast-claw/weapon")
local charged_shot = require("../navis/megaman/weapons/charged-shot/weapon")
local dust_beast_scatter = require("../navis/megaman/forms/dustcross-beast/scatter")
local falling_rock = require("../objects/falling-rock/falling_rock")
local ground_drill = require("../navis/megaman/forms/groundcross/drill")
local lockon = require("./lockon")
local slash_beast_lunge = require("../navis/megaman/forms/slashcross-beast/lunge")
local slash_charge = require("../navis/megaman/forms/slashcross/charge")
local stun_strike = require("../lib/swords/stun_strike")
local statuses = require("./status")
local supports = require("../lib/supports/controller")
local test_chips = require("../chips/test/chips")

return define.roles {
    actions = {
        forced_charged_shot = charged_shot.action,
        -- The strike the navi's request 0x80000 starts from idle (a
        -- machine sword's).
        stun_strike = stun_strike.action,
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
    chips = {
        -- What a zeroed chip field reads: the blank chip.
        zeroed = test_chips.blank,
    },
    statuses = {
        damage_word_paralysis = statuses.paralyze_90,
        counter_paralysis = statuses.paralyze_150,
        ice_freeze = statuses.freeze_150,
        hit_bug_blind = statuses.blind_1200,
        hit_bug_confuse = statuses.confuse_960,
    },
    lockon = {
        beast_claw = lockon.beast_claw,
    },
    hooks = {
        -- The NaviCust FirstBarrier, as BN6's roles fill it: the Barrier
        -- chip's barrier and its visual.
        first_barrier = function(navi: Object)
            barriers.raise(navi, barriers.barrier_10, true)
        end,
    },
}
