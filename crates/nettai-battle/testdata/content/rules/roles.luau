--!strict
-- What the rules need from the engine's test content by role
-- (docs/design/content-model-v2.md §7.4): the EXE6 overlay's modules the
-- test content has. (The test pack, crates/nettai-battle/testdata/pack,
-- brings its own.)

local absorbed_obstacle = require("../objects/absorbed_obstacle")
local barriers = require("../lib/barriers")
local beast_rush = require("./beast/rush")
local beast_out = require("../chips/beastout").beastout
local charged_shot = require("../navis/megaman/weapons/charged_shot")
local dust_beast_scatter = require("../navis/megaman/forms/dustcross_beast/scatter")
local falling_rock = require("../objects/falling_rock")
local ground_drill = require("../navis/megaman/forms/groundcross/drill")
local invalid = require("../chips/invalid").invalid
local defined = require("./role_definitions")
local slash_beast_lunge = require("../navis/megaman/forms/slashcross_beast/lunge")
local stun_strike = require("../lib/swords/stun_strike")
local statuses = require("./status")
local supports = require("../lib/supports/controller")
local test_chips = require("../chips/test")
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
        -- (The volley's role isn't filled, as in EXE6.)
        -- EXE6's Beast Out rush.
        wrapper = beast_rush.action,
        -- DustCross Beast's scatter, during which the navi isn't grounded.
        ungrounded = dust_beast_scatter.action,
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
        zeroed = test_chips["test/blank"],
        -- The custom screen's Beast Out button, as a chip in the hand, and
        -- what a selection the custom screen can't allow becomes: EXE6's.
        button_chip = beast_out,
        invalid = invalid,
    },
    statuses = {
        damage_word_paralysis = statuses.paralyze_90,
        counter_paralysis = statuses.paralyze_150,
    },
    effects = defined.effects,
    sparks = defined.sparks,
    regions = defined.regions,
    collision = defined.collision,
    sounds = defined.sounds,
    music = defined.music,
    sprites = defined.sprites,
    banners = defined.banners,
    hooks = {
        -- The first barrier, as EXE6's roles fill it: the barrier the
        -- navi's stats name, and its visual.
        first_barrier = function(navi: Object)
            local barrier = battle.navi(navi.alliance).first_barrier
            if barrier then
                barriers.raise(navi, barrier, true)
            end
        end,
    },
}
