--!strict
-- The test content's rules, their one definition (docs/design/rules-in-luau.md §2.5): EXE6's save (what a player's
-- save brings, the level its API reads), Beast Out (which the test navis' forms go by as EXE6's do), form changes,
-- emotions (Full Synchro on a counter), patch cards and dark chips (content/exe6/rules: a module the test content
-- hasn't is EXE6's), and a made-up counter of round starts, for the framework's tests of per-side state, player setups
-- and hooks. A test that needs other rules patches this module's text.

local beast = require("@self/beast")
local dark_chips = require("@self/dark_chips")
local emotion = require("@self/emotion")
local forms = require("@self/forms")
local fresh_stats = require("@self/fresh_stats")
local patch_cards = require("@self/patch_cards")
local save = require("@self/save")

export type CounterState = { starts: number, side: number, bonus: number }

-- Counts the round starts it sees, and keeps its side and twice its player's setup bonus.
local function count(side: number)
    local s = rules.state() :: CounterState
    s.starts += 1
    s.side = side
    s.bonus = (rules.setup() :: { bonus: number }).bonus * 2
end

local rules_: Rules = {
    state = {
        -- EXE6's Beast Out (its rush's, its controllers', its turn-start check's delay, its custom screen's).
        rush_phase = "u8",
        rush_started = "bool",
        rush_home_x = "u8",
        rush_home_y = "u8",
        rush_chain = "bool",
        rush_chain_block = "u8",
        rush_countdown = "u16",
        controller_step = "u8",
        controller_moves = "u16",
        controller_volley = "u16",
        controller_started = "bool",
        check_delay = "u8",
        beast_out_used = "bool",
        -- EXE6's form changes', emotions' and dark chips'.
        went_beast_out = "bool",
        crossed = "bool",
        swing_ticks = "u16",
        swung = "u8",
        bug_frags = "u32",
        -- The counter's.
        starts = "u8",
        side = "u8",
        bonus = "u16",
    },
    setup = {
        -- EXE6's: the navi, the folder, the patch cards and the NaviCust (EXE6's rules/patch_cards and rules/save
        -- read them; the test content has no NaviCust board).
        navi = "navi",
        folder = schema.list({ chip = "chip", code = "code" }, 30),
        regular_chip = "u8?",
        tag_chips = schema.list("u8", 2),
        patch_cards = schema.list("patch_card", 32),
        navicust_expansions = "u8?",
        navicust_programs = schema.list({
            program = "navicust_program",
            color = { "white", "pink", "yellow", "red", "blue", "green" },
            x = "u8",
            y = "u8",
            rotation = "u8",
            compressed = "bool",
        }, 49),
        -- EXE6's: the version, Beast Out, what the save brings, the bug frags.
        version = { "gregar", "falzar" },
        beast_out = "bool",
        hp = "u16",
        reg_up = "u8",
        sun = "bool",
        level = "u8?",
        bug_frags = "u32",
        -- EXE6's SP navi deletion times (EXE6's rules/sp_chips, which Count[SP]'s damage reads): none stated, every
        -- SP navi in no time.
        sp_times = schema.list({ chip = "chip", frames = "u16" }, 1),
        -- The counter's.
        bonus = "u8",
    },
    setup_defaults = { beast_out = true, hp = 100, reg_up = fresh_stats.reg_up, sun = false },
    extends = {
        chip = {
            -- EXE6's.
            beast = { lockon = "record:lockon", rush = "bool" },
            dark_substitute = "chip",
            hp_bug = "u8",
            sp = { by_time = "u16[11]" },
            -- Made-up fields of the game's chips, for the extension's tests.
            test_weight = "u8",
            test_tag = { level = "u8", kind = { "a", "b" } },
        },
        form = {
            -- EXE6's.
            kind = { "cross", "beast", "cross_beast", "beast_over" },
            cross_of = "navi",
            cross_release_anim = "u8",
            beast = "form",
            special_volley = "u16",
            charged_sword_rush = "bool",
        },
    },
    actions = { forms.action, beast.rush_action, beast.cross_special_action },
    buttons = { beast_out = beast.custom.button },
    windows = { beast_out = beast.custom.window, beast_out_chip = beast.custom.chip_window },
    custom = {
        open = beast.custom.open,
        confirmed = beast.custom.confirmed,
        chip_picked = beast.custom.chip_picked,
        chip_taken_back = beast.custom.chip_taken_back,
    },
    hooks = {
        round_setup = function(side: number)
            save.round_setup(side)
            patch_cards.apply(side)
            dark_chips.round_setup(side)
        end,
        round_start = function(side: number)
            count(side)
            emotion.round_start(side)
        end,
        countered = emotion.countered,
        navi_tick = emotion.navi_tick,
        turn_check = beast.turn_check,
        turn_started = beast.turn_started,
        custom_requested = beast.custom_requested,
        custom_closed = beast.custom_closed,
        chip_used = beast.chip_used,
        controller = beast.controller,
        takeover_requested = beast.takeover_requested,
        takeover = beast.takeover,
        form_reverted = beast.form_reverted,
        chip_substitute = dark_chips.chip_substitute,
        chip_prepared = dark_chips.chip_prepared,
    },
    -- Its rule sections and its roles. The made-up tables are its own (rules/panels, rules/reactions,
    -- rules/status_rules, rules/custom_screen, the lock-on's); where games differ it plays as EXE6
    -- does, and those sections are EXE6's own files (a module the test content hasn't is EXE6's): berserk, buster,
    -- chip-use, effects, elements, flow, fresh_stats, math, pools. None of its banners holds, and it has no NaviCust
    -- board.
    berserk = require("@self/berserk"),
    buster = require("@self/buster"),
    chip_use = require("@self/chip_use"),
    custom_screen = require("@self/custom_screen"),
    effects = require("@self/effects"),
    elements = require("@self/elements"),
    flow = require("@self/flow"),
    fresh_stats = fresh_stats,
    -- (Its own: no stage and no background, so no random pick.)
    link_pick = { stages = {}, first_round_stages = 0, backgrounds = {}, match_stages = {} },
    lockon = require("@self/lockon").rules,
    math = require("@self/math"),
    panels = require("@self/panels"),
    pools = require("@self/pools"),
    reactions = require("@self/reactions"),
    roles = require("@self/roles"),
    status = require("@self/status_rules"),
}

return rules_
