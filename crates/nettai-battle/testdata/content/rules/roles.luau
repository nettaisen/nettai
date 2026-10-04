--!strict
-- What the ruleset needs from the engine's test content by role
-- (docs/design/content-model-v2.md §7.4): the BN6 overlay's modules the
-- test content has. (The test pack, crates/nettai-battle/testdata/pack,
-- brings its own.)

local absorbed_obstacle = require("../objects/absorbed-obstacle/absorbed_obstacle")
local barriers = require("../lib/barriers/barriers")
local beast_rush = require("./beast/rush")
local beast_out = require("../chips/beastout/chip")
local charged_shot = require("../navis/megaman/weapons/charged-shot/weapon")
local dust_beast_scatter = require("../navis/megaman/forms/dustcross-beast/scatter")
local falling_rock = require("../objects/falling-rock/falling_rock")
local ground_drill = require("../navis/megaman/forms/groundcross/drill")
local invalid = require("../chips/invalid/chip")
local ruleset = require("./ruleset")
local slash_beast_lunge = require("../navis/megaman/forms/slashcross-beast/lunge")
local stun_strike = require("../lib/swords/stun_strike")
local statuses = require("./status")
local supports = require("../lib/supports/controller")
local test_chips = require("../chips/test/chips")
local turn = require("../navis/megaman/turn")

return {
    actions = {
        forced_charged_shot = charged_shot.action,
        -- The strike the navi's request 0x80000 starts from idle (a
        -- machine sword's).
        stun_strike = stun_strike.action,
        -- What a switched-in navi protected at the battle's end runs
        -- (GroundCross's drill).
        switch_protect = ground_drill.action,
        -- The turn L or R starts where turning is enabled.
        turn = turn.action,
        -- A switched-in navi's knock-out (SlashCross Beast's lunge).
        switch_knockout = slash_beast_lunge.action,
        -- (The volley's role isn't filled, as in BN6.)
        -- BN6's Beast Out rush.
        wrapper = beast_rush.action,
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
        -- The custom screen's Beast Out button, as a chip in the hand, and
        -- what a selection the custom screen can't allow becomes: BN6's.
        beast_out = beast_out,
        invalid = invalid,
    },
    statuses = {
        damage_word_paralysis = statuses.paralyze_90,
        counter_paralysis = statuses.paralyze_150,
        ice_freeze = statuses.freeze_150,
        hit_bug_blind = statuses.blind_1200,
        hit_bug_confuse = statuses.confuse_960,
    },
    effects = ruleset.effects,
    sparks = ruleset.sparks,
    regions = ruleset.regions,
    collision = ruleset.collision,
    sounds = ruleset.sounds,
    music = ruleset.music,
    sprites = ruleset.sprites,
    banners = ruleset.banners,
    hooks = {
        -- The first barrier, as BN6's roles fill it: the barrier the
        -- navi's stats name, and its visual.
        first_barrier = function(navi: Object)
            local barrier = battle.navi(navi.alliance).first_barrier
            if barrier then
                barriers.raise(navi, barrier, true)
            end
        end,
    },
}
