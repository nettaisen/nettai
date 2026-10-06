--!strict
-- EXE5's rules, their one definition (docs/design/rules-in-luau.md §2.5): what a side keeps through a round, what a
-- player brings, the hooks the framework calls (each says which of the game's modules it runs, in what order), the
-- custom screen's buttons and windows, EXE5's fields on its definitions, its rule sections and its roles. The modules
-- of this folder are the game's code (the save, the NaviCust, the patch cards, light and dark MegaMan, the emotions,
-- Soul Unison, the navis in auto battle Chaos Unison brings, the folder rules); its Team Battle is still to come
-- (docs/design/exe5-map.md §15).
-- Written by tools/exe5/gen_rules.py (bn6battle-verify) from the EXE5 ROM; people own it now.

local auto_battle = require("@self/auto_battle")
local emotion = require("@self/emotion")
local folder = require("@self/folder")
local fresh_stats = require("@self/fresh_stats")
local light_dark = require("@self/light_dark")
local navicust = require("@self/navicust")
local navicust_board = require("@self/navicust/board")
local patch_cards = require("@self/patch_cards")
local save = require("@self/save")
local souls = require("@self/souls")

-- The auto battle AI's state (./auto_battle/ai.luau's `State`): a navi's it drives (Dark MegaMan's, AIData +0x80 on),
-- and a side's own drive's under DarkInvs (AIData +0xF0 on), in the side's state.
local AI: StateSchema = {
    timed = "bool",
    lifetime = "u16",
    drive_ticks = "u16",
    mode = "u8",
    step = "u8",
    sub = "u8",
    misses = "u8",
    target_index = "u8",
    next_target = "u8",
    shots = "u8",
    pattern_chip = "u8",
    entry_chip = "chip",
    entry_pattern = "u8",
    since = "u32",
    hole_count = "u8",
    front_count = "u8",
    front_ticks = "u16",
    hole_ticks = "u16",
    place_step = "u8",
    place_step_init = "u8",
    place_busy = "bool",
    place_ready = "bool",
    place_chip = "chip",
    place_target = "object",
}

-- What a side keeps through a round.
local state: StateSchema = {
    -- Soul Unison (./souls): AIData +0x0F, the soul's turns left (0xFF: none to count), and Chaos Unison; the turn's
    -- arm chip (the transform record's +6; none: its 0xFFFF).
    turns = "u8",
    chaos = "bool",
    arm_chip = "chip",
    -- The custom screen's (./souls/custom.luau): the round's souls given, the soul button's offer, the soul's
    -- choice's step.
    souls_used = "u32",
    offer = "form",
    offer_chaos = "bool",
    unite_step = "u8",
    unite_count = "u8",
    -- The NaviCust's emotion-swing bug (./emotion): AIData +0x3A (ticks toward the next swing) and +0x0B (the
    -- emotion last swung to) of each navi of the player's kind, by its slot on its side's list of alive actors.
    swing_ticks = "u16[4]",
    swung = "u8[4]",
    -- The side's own drive under DarkInvs (./auto_battle): the AI's state, as a navi it drives keeps it.
    drive = AI,
    -- The side's auto battle data as its console sent it as the battle started (./auto_battle/block.luau), which a
    -- navi in auto battle plays and turns.
    auto_battle_places = schema.list(auto_battle.block.PLACE, auto_battle.block.PLACES),
    -- What the souls' additions to the custom screen keep of one screen: each soul's fields, in its folder
    -- (navis/megaman/forms/<soul>), which ./souls/custom gathers.
    screen = souls.custom.screen,
}

local rules: Rules = {
    state = state,
    -- What a player brings (nettai-match from a match's side, exe5-compat from a recording): a side's facts.
    setup = {
        -- The navi they play: the round's stats are its (`Battle::new` holds them to it). No default: a side states
        -- it.
        navi = "navi",
        -- Their folder as the save keeps it, an entry with no chip empty (a folder being made); the Regular chip, an
        -- entry of it (from 0), dealt first. (EXE5's folders have no tag chips.) A tool deals the round's battle folder
        -- from them; ./folder checks them (`validate`).
        folder = schema.list({ chip = "chip", code = "code" }, 30),
        regular_chip = "u8?",
        -- The patch cards they have installed, in their list's order (./patch_cards): each applies (a save's card
        -- switched off is none of the side's: it does nothing in battle).
        patch_cards = schema.list("patch_cards", 32),
        -- Their NaviCust (./navicust): how far its board has been expanded (none: no NaviCust, the stats as the
        -- setup gives them: a navi but MegaMan's, a recording's), and the programs placed on its 7x7 grid in its
        -- list's order, each in one of its colors (its definition's `colors`), its center at (x, y), turned
        -- `rotation` quarters clockwise, compressed or not.
        navicust_expansions = "u8?",
        navicust_programs = schema.list({
            program = "navicust_programs",
            color = { "white", "pink", "yellow", "red", "blue", "green" },
            x = "u8",
            y = "u8",
            rotation = "u8",
            compressed = "bool",
        }, 49),
        -- Their auto battle data as their save holds it (./auto_battle/block.luau: the block's 42 places and its eight
        -- pattern records), which a navi in auto battle plays of them as their console sends it; `auto_battle_sent`:
        -- the places are as the console sent them, a recording's (the round's setup sends nothing).
        auto_battle_places = schema.list(auto_battle.block.PLACE, auto_battle.block.PLACES),
        auto_battle_records = schema.list(auto_battle.block.RECORD, auto_battle.block.RECORDS),
        auto_battle_sent = "bool",
        -- What the save brings to the navi's stats (./save): MegaMan's base HP, the Regular memory, and a team navi's
        -- level (the story's progress).
        hp = "u16",
        reg_up = "u8",
        level = "u8?",
        -- MegaMan's light/dark value (NaviStats +0x44, ./light_dark).
        karma = "u16",
        -- The souls the save has (./souls; a match's side's `souls`, a recording's its version's), which the custom
        -- screen's soul button may offer, as the save's soul flags (0x08024BF0's, one a soul; any souls of the game,
        -- either version's); Soul Unison (event flag 0: the soul button) and Chaos Unison (event flag 0x236: a dark
        -- chip's soul).
        souls = "form[16]",
        soul_unison = "bool",
        chaos_unison = "bool",
        -- How long the save took to delete each SP navi (./sp_chips: the save's 0x0203E560), by its SP chip, in
        -- frames.
        sp_times = schema.list({ chip = "chip", frames = "u16" }, 18),
    },
    -- A setup that says nothing: a new game's base HP, 100 (MegaMan's fresh HP), and Regular memory (the fresh
    -- stats'), no level (MegaMan's); a fresh save's light/dark value (0x08010C00 writes 500: light for the chips, no
    -- holy panels cleared, the starting mood 0x80); a finished save's souls, every one of either version (by id: a
    -- form's module reads ./souls, so this names them rather than reads theirs), and both unisons.
    setup_defaults = {
        -- A NaviCust with no programs on the largest board (MegaMan's compiles one; a recording's boundary states
        -- none, its stats already the NaviCust's).
        navicust_expansions = #navicust_board.boards - 1,
        -- What the game's battle end writes of a player it has learned nothing of: no places, every record zeros.
        auto_battle_records = auto_battle.block.nothing_learned(),
        hp = save.NEW_GAME_HP,
        reg_up = fresh_stats.reg_up,
        karma = 500,
        souls = {
            "colonelsoul",
            "gyrosoul",
            "knightsoul",
            "magnetsoul",
            "meddysoul",
            "napalmsoul",
            "numbersoul",
            "protosoul",
            "searchsoul",
            "shadowsoul",
            "toadsoul",
            "tomahawksoul",
        },
        soul_unison = true,
        chaos_unison = true,
        -- Every SP navi deleted in no time (its chip's best damage), in the save's order.
        sp_times = {
            { chip = "protomn-sp", frames = 0 },
            { chip = "gyroman-sp", frames = 0 },
            { chip = "serchmn-sp", frames = 0 },
            { chip = "naplmmn-sp", frames = 0 },
            { chip = "magntmn-sp", frames = 0 },
            { chip = "meddy-sp", frames = 0 },
            { chip = "colonel-sp", frames = 0 },
            { chip = "shadomn-sp", frames = 0 },
            { chip = "numbrmn-sp", frames = 0 },
            { chip = "tmhwkmn-sp", frames = 0 },
            { chip = "knigtmn-sp", frames = 0 },
            { chip = "toadman-sp", frames = 0 },
            { chip = "shademn-sp", frames = 0 },
            { chip = "blizman-sp", frames = 0 },
            { chip = "cloudmn-sp", frames = 0 },
            { chip = "cosmomn-sp", frames = 0 },
            { chip = "larkman-sp", frames = 0 },
            { chip = "gridman-sp", frames = 0 },
        },
    },
    -- The state of each navi no player controls that the rules drive (./auto_battle).
    navi_state = AI,
    -- EXE5's fields on its definitions (docs/design/rules-in-luau.md §7.5), which the engine checks and reads none
    -- of. What a soul's form says for the rules that ask (a soul is named by its form's id; the original tests its
    -- number, NaviStats +0x2C, in each place; the pack's types.d.luau types them): its navi's image at the change
    -- (./souls/image, a `SoulImage`) and what it adds to the custom screen (./souls/custom, a `SoulCustom`); its
    -- sword's arm, the blade's animation (lib/swords, 0x080EC038); its charged sword warping behind the enemy
    -- whatever the charge (lib/stepsword, 0x08012538); VarSwrd's wait for its sequence while A is up (chips/varswrd,
    -- 0x080ED570); the cameras' shake as MegaMan emerges in it (./souls/change, 0x0801212C).
    extends = {
        chip = {
            -- An SP navi chip's damage by its deletion-time step, and in the own-gauges mode (./sp_chips).
            sp = { by_time = "u16[11]", operation_battle = "u16" },
        },
        form = {
            image = "record:soul-image",
            custom = "record:soul-custom",
            blade_anim = "u8",
            steps_behind = "bool",
            var_sword_waits = "bool",
            emerge_shake = "bool",
        },
    },
    -- Its own actions, which reach its state: a soul's change and revert (./souls), Dark MegaMan's leave
    -- (./auto_battle).
    actions = { souls.change_action, souls.revert_action, auto_battle.leave },
    -- The custom screen's buttons and windows: Soul Unison's, and each soul's own (./souls/custom.luau).
    buttons = souls.custom.buttons,
    windows = souls.custom.windows,
    custom = {
        -- The deal: a worried or dark MegaMan's dark chip offer (./light_dark), then the souls' (MeddySoul's
        -- capsules).
        deal = function(side: number)
            light_dark.deal(side)
            souls.custom.deal(side)
        end,
        hand_size = souls.custom.hand_size,
        open = souls.custom.open,
        confirmed = souls.custom.confirmed,
    },
    hooks = {
        -- What the save brings to the navi's stats; then the NaviCust's compile, which reads them; then the patch
        -- cards, which apply to what it made; then Hub Style's faces and name (./light_dark).
        round_setup = function(side: number)
            save.round_setup(side)
            navicust.round_setup(side)
            patch_cards.apply(side)
            light_dark.round_setup(side)
            -- The auto battle data the console sends (./auto_battle/block.luau).
            auto_battle.block.send(side)
        end,
        -- Light and dark MegaMan (./light_dark): his starting mood, palette, a hit's bug, the holy panels a dark
        -- one clears, the chips each may use and a dark chip's cost.
        starting_mood = light_dark.starting_mood,
        navi_palette = light_dark.navi_palette,
        navi_bug = light_dark.navi_bug,
        navi_intake = light_dark.navi_intake,
        chip_cost = light_dark.chip_cost,
        chip_check = light_dark.chip_check,
        -- Full Synchro on a counter, and the NaviCust's swing bug (./emotion).
        round_start = emotion.round_start,
        countered = emotion.countered,
        -- Each navi's tick: the swing bug, then a driven navi's time (./auto_battle).
        navi_tick = function(side: number, navi: Object)
            emotion.navi_tick(side, navi)
            auto_battle.navi_tick(side, navi)
        end,
        -- Soul Unison's turns (./souls): the turn's request read (ColonelSoul's arm chip), the revert when they run
        -- out, the count down as a custom screen is asked for.
        turn_opened = souls.custom.turn_opened,
        turn_check = souls.turn_check,
        custom_requested = souls.custom_requested,
        -- The navis in auto battle Chaos Unison brings, and a side's own drive under DarkInvs (./auto_battle).
        controller = auto_battle.controller,
        -- The folder's rules, which tools ask (the simulation never does).
        folder_check = folder.check,
        -- What is wrong with a side's setup, which tools ask (the simulation never does): what the save brings, the
        -- patch cards, the NaviCust, the folder, the auto battle data.
        validate = function(side: number): { SetupProblem }
            local out: { SetupProblem } = {}
            local function problem(text: string, field: string?, entry: number?)
                table.insert(out, { text = text, field = field, entry = entry })
            end
            save.validate(side, problem)
            patch_cards.validate(side, problem)
            navicust.validate(side, problem)
            folder.validate(side, problem)
            auto_battle.block.validate(side, problem)
            return out
        end,
    },
    -- Its rule sections (each a plain table, which the engine reads by its schema) and its roles.
    banners = require("@self/banners"),
    buster = require("@self/buster"),
    chip_use = require("@self/chip_use"),
    custom_screen = require("@self/custom_screen"),
    effects = require("@self/effects"),
    elements = require("@self/elements"),
    flow = require("@self/flow"),
    fresh_stats = fresh_stats,
    link_pick = require("@self/link_pick"),
    math = require("@self/math"),
    -- What a tool shows of a side's setup, pane by pane (./panes).
    panes = require("@self/panes"),
    panels = require("@self/panels"),
    pools = require("@self/pools"),
    reactions = require("@self/reactions"),
    roles = require("@self/roles"),
    status = require("@self/status").rules,
}

return rules
