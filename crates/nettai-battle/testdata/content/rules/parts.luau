--!strict
-- The test content's rules (docs/design/rules-in-luau.md): its made-up
-- parts and its one rules definition, for the framework's tests of per-side
-- state, player setups and hooks. A test that plays by other parts (the
-- marker, the watcher, EXE6's patch cards part) writes its list in place of
-- `PARTS` (testing.rs's `with_parts`).

local compose = require("@exelib/rules/compose")
local patch_cards = require("./patch_cards")

export type CounterState = { starts: number, side: number, bonus: number }

-- Counts the round starts it sees, and keeps its side and twice its
-- player's setup bonus.
local counter: RulesPart = {
    state = { starts = "u8", side = "u8", bonus = "u16" },
    setup = { bonus = "u8" },
    -- Made-up fields of the game's chips, for the extension's tests.
    extends = {
        chip = {
            test_weight = "u8",
            test_tag = { level = "u8", kind = { "a", "b" } },
        },
    },
    hooks = {
        round_start = function(side: number)
            local s = rules.state() :: CounterState
            s.starts += 1
            s.side = side
            s.bonus = (rules.setup() :: { bonus: number }).bonus * 2
        end,
    },
}

export type MarkerState = { mark: number }

-- Marks its side's state, to tell the rulesets apart.
local marker: RulesPart = {
    state = { mark = "u8" },
    hooks = {
        round_start = function(side: number)
            local s = rules.state() :: MarkerState
            s.mark = 0x40 + side
        end,
    },
}

export type WatcherState = { intakes: number, x: number, y: number }

-- Counts its side's navi intakes and keeps the navi's panel; refuses the
-- test bomb, giving the test seed instead.
local test_chips = require("../chips/test")
local watcher: RulesPart = {
    state = { intakes = "u16", x = "u8", y = "u8" },
    hooks = {
        navi_intake = function(_side: number, navi: Object)
            local s = rules.state() :: WatcherState
            s.intakes += 1
            s.x, s.y = navi.panel_x, navi.panel_y
        end,
        chip_check = function(_side: number, _navi: Object, chip: Chip?): Chip?
            if chip == test_chips.bomb then
                return test_chips.seed
            end
            return nil
        end,
    },
}

-- EXE6's own save (content/exe6/rules/save: what a player's save brings,
-- the level its API reads), Beast Out turns (content/exe6/rules/beast),
-- which the test navis' forms go by as EXE6's do, its emotions
-- (content/exe6/rules/emotion: Full Synchro on a counter) and its dark
-- chips' bug frags (content/exe6/rules/dark_chips).
local beast = require("./beast")
local save = require("./save")
local dark_chips = require("./dark_chips")
local emotion = require("./emotion")
local forms = require("./forms")

-- The parts the test content plays by (EXE6's save and Beast Out first, then
-- the counter, then EXE6's forms, emotions and dark chips).
local PARTS: { RulesPart } = { save, beast, counter, forms.part, emotion.part, dark_chips }

return {
    counter = counter,
    marker = marker,
    watcher = watcher,
    patch_cards = patch_cards,
    -- The test content's rules: its parts, its rule sections and its
    -- roles. The rules state every rule of their game. The made-up tables
    -- are its own (rules/panels, rules/reactions, rules/status_rules,
    -- rules/custom_screen, rules/sp_chips, the lock-on's); where games
    -- differ it plays as EXE6 does, and those sections are EXE6's own files
    -- (a module the test content hasn't is EXE6's): berserk, buster,
    -- chip-use, effects, elements, flow, fresh_stats, math, pools. None of
    -- its banners holds, and it has no NaviCust board.
    ruleset = define.rules(compose.rules(PARTS, {
        berserk = require("./berserk"),
        buster = require("./buster"),
        chip_use = require("./chip_use"),
        custom_screen = require("./custom_screen"),
        effects = require("./effects"),
        elements = require("./elements"),
        flow = require("./flow"),
        fresh_stats = require("./fresh_stats"),
        -- (Its own: no stage and no background, so no random pick.)
        link_pick = { stages = {}, first_round_stages = 0, backgrounds = {} },
        lockon = require("./lockon").rules,
        math = require("./math"),
        panels = require("./panels"),
        pools = require("./pools"),
        reactions = require("./reactions"),
        roles = require("./roles"),
        sp_chips = require("./sp_chips"),
        status = require("./status_rules"),
    })),
}
