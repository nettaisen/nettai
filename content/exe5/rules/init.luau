--!strict
-- EXE5's rules, their one definition (docs/design/rules-in-luau.md §2.5): what a side keeps through a round, what a
-- player brings, the hooks the framework calls (each says which of the game's modules it runs, in what order), the
-- custom screen's buttons and windows, EXE5's fields on its definitions, its rule sections and its roles. The modules
-- of this folder are the game's code (the save, the NaviCust, the patch cards, light and dark MegaMan, the emotions,
-- Soul Unison, the navis in auto battle Chaos Unison brings, the folder rules); its Team Battle is still to come
-- (docs/design/exe5-map.md §15).
-- Written by tools/exe5/gen_rules.py (bn6battle-verify) from the EXE5 ROM; people own it now.

local auto_battle = require("@self/auto_battle")
local dark_survival = require("@self/emotion/dark_survival")
local double_attacks = require("@self/double_attacks")
local emotion = require("@self/emotion")
local folder = require("@self/folder")
local fresh_stats = require("@self/fresh_stats")
local chaos_charge = require("@self/souls/chaos_charge")
local light_dark = require("@self/light_dark")
local navicust = require("@self/navicust")
local navicust_board = require("@self/navicust/board")
local navicust_bugs = require("@self/navicust/bugs")
local party = require("@self/party")
local patch_cards = require("@self/patch_cards")
local colonel_support = require("./navis/colonel/support")
local gyroman_support = require("./navis/gyroman/support")
local napalmman_support = require("./navis/napalmman/support")
local numberman_dice_bonus = require("./navis/numberman/dice_bonus")
local protoman_support = require("./navis/protoman/support")
local tomahawkman_support = require("./navis/tomahawkman/support")
local toadman_rescue = require("./navis/toadman/rescue")
local save = require("@self/save")
local shout = require("@self/shout")
local soldier = require("./objects/soldier")
local souls = require("@self/souls")

-- What the tools read of the game's builds, as data (./builds: the versions, a new save's folder): loaded with the
-- rules, which read none of it.
require("@self/builds")
-- And what a ruleset may hold back of a build, and the game's ready-made rulesets (./rulesets), likewise.
require("@self/rulesets")

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
    turns = schema.role("form_offer.turns", "u8"),
    chaos = "bool",
    arm_chip = "chip",
    -- The custom screen's (./souls/custom.luau): the round's souls given, the soul button's offer, the soul's
    -- choice's step.
    souls_used = "u32",
    offer = schema.role("form_offer.form", "form"),
    offer_chaos = schema.role("form_offer.alternate", "bool"),
    unite_step = schema.role("offer_flight.step", "u8"),
    unite_count = schema.role("offer_flight.count", "u8"),
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
    -- Chaos Unison's charge (./souls/chaos_charge): AIData +0x6C, the chaos level; +0x1C and +0x1F, the cycle's counter
    -- and window; the release's outcome as it was released.
    chaos_level = "u8",
    chaos_counter = "u8",
    chaos_window = "u8",
    chaos_success = "bool",
    -- The last navi chip used, of either side (0x0203C430, cleared as the battle starts): the navi chip controllers
    -- keep a chip of the navi chips' block as its navi comes, in both sides' states (./api); DethPhnx brings its navi
    -- again.
    last_navi_chip = { chip = "chip", element = "u8", damage = "u32" } :: StateSchema,
    -- The party battle's Double Attack (./double_attacks): AIData +0x73, spent this round (its 0xFF), else on a paused
    -- switch the member the follow-up launches with.
    double_attack = { spent = "bool", partner = "u8" } :: StateSchema,
    -- The party battle's (./party): ToadMan's rescue is spent this battle (the DS's party block +0xB).
    rescued = "bool",
    -- EXE5 DS's shout is spent this battle (./shout: its battle flag 0x800 or 0x1000).
    shouted = "bool",
    -- NumberMan's DiceBons is spent this battle (navis/numberman/dice_bonus: the party block's +0x13).
    dice_bonus = "bool",
    -- The party's supports' (./party's `Supports`: the party block's other fields).
    supports = {
        protoman_used = "bool",
        protoman_count = "u8",
        gyroman_spawned = "bool",
        napalmman_set = "bool",
        napalmman_timer = "u16",
        colonel_used = "bool",
        colonel_count = "u8",
        tomahawkman_timer = "u16",
        tomahawkman_set = "bool",
        busy = "bool[3]",
        shadowman_used = "bool",
        megaman_timer = "u16",
        megaman_set = "bool",
        searchman_used = "bool",
        gyroman_gust = "object",
        knightman_guard = "object",
    } :: StateSchema,
}

local rules: Rules = {
    state = state,
    -- A navi's stats of EXE5's own, beside the engine's (NaviStats' bytes, which exe5-compat maps by these names):
    -- fighting in the sun (+0x22, MegaMan's: ./save; GunDelSol reads it), what the NaviCust does to chip drops and
    -- random encounters (+0x26, +0x28: the patch cards' bug count reads them), Hub Style (+0x4C, which its patch card
    -- 111, 0x6F, sets when installed and on, 0x08138214: 1 by Team ProtoMan's card, 2 by Team Colonel's, 0x081382FC;
    -- MegaMan's buster, arm, shade, palettes and faces read it), and the turns Soul Unison gives a soul beside its 3
    -- (+0x32, signed: the NaviCust's SoulT+1, the patch cards' SoulTm+ and SoulTm-; ./souls reads it at OK,
    -- 0x08024FF6).
    stats = {
        sun = "bool",
        chip_drops = "u8",
        encounters = "u8",
        hub_style = "u8",
        soul_turn_bonus = "i8",
        -- The party battle's (EXE5 DS: ./party): the switches the side may make beside the DS's 3 (+0x17 less 3, which
        -- its fresh stats write and the GBA's don't: NaviCust's NavChg+1 and NavChg+2 raise it, to 5 in all), the
        -- bonus of a team navi's own chip's damage (+0x56: PartyCust's P.Chp, 0x0203BF2C adds it), and whether the
        -- navi may support its party while benched (+0x59: NaviCust's Spport for MegaMan, PartyCust's P.Spport for a
        -- team navi; 0x02004E9C reads it).
        switch_bonus = "u8",
        chip_attack_bonus = "u16",
        supports_party = "bool",
    },
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
        -- switched off is none of the side's: it does nothing in battle). A card each team's ROMs give effects of
        -- their own (card 111, Bass-Cross MegaMan) has the variant chosen for it, a team's (its effects' `variant`s),
        -- as a NaviCust program's color is chosen; a card without variants, none (left out).
        patch_cards = schema.list({ card = "patch_cards", variant = { "protoman", "colonel" } }, 32),
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
        -- pattern records), which a navi in auto battle plays of them as their console sends it.
        auto_battle_places = schema.list(auto_battle.block.PLACE, auto_battle.block.PLACES),
        auto_battle_records = schema.list(auto_battle.block.RECORD, auto_battle.block.RECORDS),
        -- What the save brings to the navi's stats (./save): MegaMan's base HP, the Regular memory, fighting in the
        -- sun (+0x22), and a team navi's level (the story's progress). A side that states none has the highest
        -- (`setup_defaults`; a team navi its story's last level, which the tools state, MegaMan none).
        hp = "u16",
        reg_up = "u8",
        sun = "bool",
        level = "u8?",
        -- MegaMan's light/dark value (NaviStats +0x44, ./light_dark).
        karma = "u16",
        -- The souls the save has (./souls; a match's side's `souls`, a recording's its version's), which the custom
        -- screen's soul button may offer, as the save's soul flags (any souls of the game, either version's): as many
        -- as the button's list holds, twelve (0x08024B28 goes through the twelve families of 0x08024BE0, its soul
        -- flags 0x08024BF0's, one a soul: `cmp r6, #13`); none, no soul button, as a save without Soul Unison (event
        -- flag 0) has; and Chaos Unison (event flag 0x236: a dark chip's soul).
        souls = "form[12]",
        chaos_unison = "bool",
        -- How long the save took to delete each SP navi (./sp_chips: the save's 0x02002670, a battle's copy
        -- 0x0203E560), by its SP chip, in frames.
        sp_times = schema.list({ chip = "chip", frames = "u16" }, 18),
        -- The team navis MegaMan brings to a party battle (./party, EXE5 DS's: the save's team, or Navi Select's two in a
        -- link battle), in order, each with its PartyCust programs (./party/partycust: no more than its gauge holds,
        -- ten blocks at most). None: no party.
        party = schema.list({ navi = "navi", programs = schema.list("partycust_programs", 10) }, 2),
    },
    -- A setup that says nothing: what every save brings at its highest, the base HP 1000, the Regular memory 50 and
    -- the sun, no level (MegaMan's); a fresh save's light/dark value (0x08010C00 writes 500: light for the chips, no
    -- holy panels cleared, the starting mood 0x80); a finished save's souls, every one of either version (by id: a
    -- form's module reads ./souls, so this names them rather than reads theirs), and Chaos Unison.
    setup_defaults = {
        -- A NaviCust with no programs on the largest board (MegaMan's compiles one; a recording's boundary states
        -- none, its stats already the NaviCust's).
        navicust_expansions = #navicust_board.boards - 1,
        -- What the game's battle end writes of a player it has learned nothing of: no places, every record zeros.
        auto_battle_records = auto_battle.block.nothing_learned(),
        hp = save.MAX_HP,
        reg_up = save.MAX_REG_UP,
        sun = true,
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
            -- What the auto battle AI reads of it (./auto_battle/place; the record's byte 13 and 23): how the navi gets in
            -- place for it, its positioning class, and for the classes that read one its reach list (./auto_battle/data's,
            -- by name) or its spot test. A chip without it the AI can't play (its class past the table: the original
            -- crashes).
            -- A dark chip's cost to its user (./light_dark, 0x0802C934's entry: a routine of 0x0802C974's,
            -- lib/dark_costs), a capsule's mix into the chip it goes into (navis/megaman/forms/meddysoul/capsules,
            -- 0x08023824: the hand's modifier bits), and a chip past the chip pack's list (./folder: its last chip
            -- 0x13A; the BatCans).
            dark_cost = "record:exe5-dark-cost",
            -- The Double Attack a chip of the DS's is (lib/double_attack; ./double_attacks fires it).
            double_attack = "record:exe5-double-attack",
            capsule_mix = "u8",
            past_the_pack = "bool",
            -- A chip a save can collect though its record has no library flag (./folder: the Giga chips Team Colonel's
            -- library lists, and the Japanese game's gifts, LeadRaid and ChaosLrd).
            collectible = "bool",
            -- The version whose saves hold the chip (a team's own Giga chips: each version's library lists its own
            -- six, and a Giga chip isn't traded), which a ruleset's groups go by (./builds' `groups`). None: either
            -- team's (ChaosLrd, which both Japanese games gave out).
            version = { "protoman", "colonel" },
            auto_battle = {
                class = {
                    "into_reach", "to_front", "spot_test", "landing_ahead", "behind_target", "three_ahead_near_enemy",
                    "lava_ahead", "gauge_into_reach", "sword_swing", "when_enemy_in_area", "at_once", "before_target",
                    "when_hole_in_area", "three_ahead_open", "when_area_lost", "when_panel_changed",
                    "when_area_whole", "when_visible", "when_unbarriered", "when_enemy_barrier", "own_panel_ahead",
                    "when_trap_free", "when_any_sea", "when_any_lava", "when_any_hole", "to_back", "hurt_into_reach",
                    "when_bugged", "when_any_field_object", "when_free_ahead_of_enemies", "to_special_panel",
                },
                range = {
                    "ahead_then_column", "two_ahead", "column_two_ahead", "three_in_row", "three_ahead", "one_ahead",
                    "column_ahead", "two_in_row", "column_three_ahead", "two_columns_ahead", "columns_two_to_four",
                    "column_ahead_and_two", "columns_two_and_three", "x_one_ahead", "ahead_in_row",
                    "cross_three_ahead", "around", "columns_three_to_five", "ring_three_ahead", "all_ahead",
                    "target_at_back_or_edge", "behind_target_up_open",
                },
                -- A `sword_swing` chip's swings' reach lists, by the side's sword pick (0x0802A3C8: VarSwrd's six
                -- from its byte 0, NeoVari's four from byte 6), and an element trap, which a `when_trap_free` chip
                -- may set over (0x0802A9B4: chips 0x8C to 0x8F).
                swings = {
                    {
                        "ahead_then_column", "two_ahead", "column_two_ahead", "three_in_row", "three_ahead", "one_ahead",
                        "column_ahead", "two_in_row", "column_three_ahead", "two_columns_ahead", "columns_two_to_four",
                        "column_ahead_and_two", "columns_two_and_three", "x_one_ahead", "ahead_in_row",
                        "cross_three_ahead", "around", "columns_three_to_five", "ring_three_ahead", "all_ahead",
                    },
                },
                element_trap = "bool",
            },
        },
        navi = {
            -- The team whose saves hold a team navi (./builds' groups: a ruleset of one version per build holds a
            -- party to one team's), and what it is in a party (./party, EXE5 DS's party battle): its PartyCust gauge's
            -- blocks, its base HP and its own chip's damage by the side's level, 0 to 9.
            version = { "protoman", "colonel" },
            party = { gauge = "u8", hp = "u16[10]", chip_damage = "u16[10]" },
        },
        form = {
            -- A soul (./souls): the family of the chips given up for it (0x08024BE0's table). A soul is named by its
            -- form's key; the original's number for it (NaviStats +0x2C) is compat's.
            soul = {
                family = {
                    "sword", "wind", "cursor", "fire", "elec", "recovery", "summon", "invisible", "plus", "aqua",
                    "break", "wood",
                },
            },
            image = "record:soul-image",
            custom = "record:soul-custom",
            blade_anim = "u8",
            steps_behind = "bool",
            var_sword_waits = "bool",
            emerge_shake = "bool",
            -- A soul's Chaos Unison: the weapon its charge fires, its own cycle (./souls/chaos_charge: the period,
            -- then the windows' bounds; none, the chaos level's row), and the blade its swords hold while the charge
            -- is armed (../lib/swords' `held_blade`; none, the swords' own).
            chaos = { weapon = "weapon", cycle = "u8[4]", blade = "record:attachment-look" },
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
            -- The party's partners join (./party).
            party.round_setup(side)
            -- The auto battle data both consoles send, shuffled alike, once for the round (./auto_battle/block.luau).
            if side == 0 then
                auto_battle.block.send()
            end
        end,
        -- Light and dark MegaMan (./light_dark): his starting mood, palette, a hit's bug (its codes first, then the
        -- NaviCust's: ./navicust/bugs), the holy panels a dark one clears, the chips each may use and a dark chip's
        -- cost.
        starting_mood = light_dark.starting_mood,
        navi_palette = light_dark.navi_palette,
        navi_bug = function(side: number, navi: Object): ("edited" | "spared")?
            return light_dark.navi_bug(side, navi) or navicust_bugs.navi_bug(side, navi)
        end,
        -- The NaviCust's hit bug, whenever a hit asks a flinch or a drag (./navicust/bugs).
        navi_flinched = navicust_bugs.navi_flinched,
        navi_intake = light_dark.navi_intake,
        chip_cost = light_dark.chip_cost,
        chip_check = light_dark.chip_check,
        cut_in_barred = light_dark.cut_in_barred,
        -- A loss of HP to 0: a dark MegaMan's last stand (./emotion/dark_survival).
        hp_emptied = dark_survival.hp_emptied,
        -- At 0 HP where the navi would be deleted: ToadMan's rescue in a party battle (navis/toadman/rescue).
        navi_falling = toadman_rescue.navi_falling,
        -- A party battle's switch requests (./party): the bottom screen's panel's refusals, and the battle mode where
        -- a request isn't a switch.
        switch_requested = party.switch_requested,
        switch_asked = party.switch_asked,
        member_busy = party.member_busy,
        -- The party's supports (0x02003764), each fight tick, in a link battle for both sides and offline for side 0
        -- alone; their turn's flags start over as the fight starts (./party).
        fight_tick = function(side: number)
            if not battle.link() and side ~= 0 then
                return
            end
            protoman_support.tick(side)
            gyroman_support.tick(side)
            napalmman_support.tick(side)
            colonel_support.tick(side)
            tomahawkman_support.tick(side)
        end,
        fight_started = party.fight_started,
        -- EXE5 DS's shout into the microphone (./shout), which nothing in nettai makes.
        mic_heard = shout.mic_heard,
        -- A chip attack's end: NumberMan's DiceBons in a party battle (navis/numberman/dice_bonus).
        chip_ended = numberman_dice_bonus.chip_ended,
        -- A hit on the navi shows: MegaMan's hurt voice's draw in a party battle (./party).
        navi_hit_shown = party.navi_hit_shown,
        -- An obstacle where ColonelSoul's army can use it turns into a soldier (objects/soldier).
        obstacle_reaction = soldier.obstacle_reaction,
        -- Full Synchro on a counter, and the NaviCust's swing bug (./emotion).
        round_start = emotion.round_start,
        countered = emotion.countered,
        -- Each navi's tick: the swing bug, then a driven navi's time (./auto_battle).
        navi_tick = function(side: number, navi: Object)
            chaos_charge.navi_tick(side, navi)
            emotion.navi_tick(side, navi)
            auto_battle.navi_tick(side, navi)
        end,
        -- A party's switch settled: the DS's Double Attack, at once in battle, by the follow-up while paused
        -- (./double_attacks).
        navi_switched = double_attacks.navi_switched,
        -- Chaos Unison's charge released, and taken from idle (./souls/chaos_charge).
        charge_released = chaos_charge.charge_released,
        release_taken = chaos_charge.release_taken,
        -- Soul Unison's turns (./souls): the turn's request read (ColonelSoul's arm chip), the revert when they run
        -- out, the count down as a custom screen is asked for.
        turn_opened = souls.custom.turn_opened,
        turn_check = souls.turn_check,
        custom_requested = souls.custom_requested,
        -- The navis in auto battle Chaos Unison brings, and a side's own drive under DarkInvs (./auto_battle).
        controller = auto_battle.controller,
        -- The folder's rules, which tools ask (the simulation never does): the rules a folder breaks, and what they
        -- allow of one.
        folder_check = folder.check,
        folder_view = folder.view,
        -- What a tool shows of the side's setup that the rules alone know: what gives the navi its abilities, its
        -- bugs, the patch cards' MB; and a Soul Unison's turns, a stat of their own (./souls/custom).
        overview = function(side: number): SetupOverview?
            local out: SetupOverview = { abilities = {}, bugs = {}, lists = {} }
            navicust.overview(side, out)
            patch_cards.overview(side, navicust.card_bug, out)
            souls.custom.overview(side, out)
            party.overview(side, out)
            return out
        end,
        -- What is wrong with a side's setup, which tools ask (the simulation never does): what the save brings, the
        -- patch cards, the NaviCust, the folder, the auto battle data.
        validate = function(side: number): { SetupProblem }
            local out: { SetupProblem } = {}
            local function problem(p: SetupProblem)
                table.insert(out, p)
            end
            save.validate(side, problem)
            patch_cards.validate(side, problem)
            navicust.validate(side, problem)
            folder.validate(side, problem)
            auto_battle.block.validate(side, problem)
            party.validate(side, problem)
            return out
        end,
    },
    -- Its rule sections (each a plain table, which the engine reads by its schema) and its roles.
    banners = require("@self/banners"),
    -- The songs a player may pick as a battle's music (./battle_themes), which the battle plays none of.
    battle_themes = require("@self/battle_themes"),
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
    panels = require("@self/panels"),
    pools = require("@self/pools"),
    reactions = require("@self/reactions"),
    roles = require("@self/roles"),
    status = require("@self/status").rules,
}

return rules
