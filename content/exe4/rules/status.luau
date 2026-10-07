--!strict
-- Statuses (0x08018550, EXE6's `off_80209EC`: four groups, paralysis, confusion, blindness and immobilization,
-- four levels each where EXE5's have six; their bytes are compat's): what a hit's status does to its target. The
-- timers are at the collision record's +0x10 to +0x16. Rows past a group's end read the next groups' rows, or the
-- bytes after the table: named for what they read, as EXE5's.
-- Written by tools/exe4/gen_rules.py (bn6battle-verify) from the EXE4 ROM; people own it now.

local status = {
    paralyze_90 = new.status {
        id = "paralyze-90",
        requests = 0x00000008,
        duration = 90,
        timer = "paralyze",
    },
    paralyze_120 = new.status {
        id = "paralyze-120",
        requests = 0x00000008,
        duration = 120,
        timer = "paralyze",
    },
    paralyze_150 = new.status {
        id = "paralyze-150",
        requests = 0x00000008,
        duration = 150,
        timer = "paralyze",
    },
    paralyze_4 = new.status {
        id = "paralyze-4",
        requests = 0x00000008,
        duration = 4,
        timer = "paralyze",
    },
    confuse_480_past_paralyze = new.status {
        id = "confuse-480-past-paralyze",
        requests = 0x00000080,
        duration = 480,
        timer = "confuse",
    },
    confuse_720_past_paralyze = new.status {
        id = "confuse-720-past-paralyze",
        requests = 0x00000080,
        duration = 720,
        timer = "confuse",
    },
    confuse_960_past_paralyze = new.status {
        id = "confuse-960-past-paralyze",
        requests = 0x00000080,
        duration = 960,
        timer = "confuse",
    },
    confuse_4_past_paralyze = new.status {
        id = "confuse-4-past-paralyze",
        requests = 0x00000080,
        duration = 4,
        timer = "confuse",
    },
    blind_480_past_paralyze = new.status {
        id = "blind-480-past-paralyze",
        requests = 0x00000020,
        duration = 480,
        timer = "blind",
    },
    blind_720_past_paralyze = new.status {
        id = "blind-720-past-paralyze",
        requests = 0x00000020,
        duration = 720,
        timer = "blind",
    },
    blind_1200_past_paralyze = new.status {
        id = "blind-1200-past-paralyze",
        requests = 0x00000020,
        duration = 1200,
        timer = "blind",
    },
    blind_4_past_paralyze = new.status {
        id = "blind-4-past-paralyze",
        requests = 0x00000020,
        duration = 4,
        timer = "blind",
    },
    immobilize_2_past_paralyze = new.status {
        id = "immobilize-2-past-paralyze",
        requests = 0x00000040,
        duration = 2,
        timer = "immobilize",
    },
    immobilize_60_past_paralyze = new.status {
        id = "immobilize-60-past-paralyze",
        requests = 0x00000040,
        duration = 60,
        timer = "immobilize",
    },
    immobilize_120_past_paralyze = new.status {
        id = "immobilize-120-past-paralyze",
        requests = 0x00000040,
        duration = 120,
        timer = "immobilize",
    },
    immobilize_4_past_paralyze = new.status {
        id = "immobilize-4-past-paralyze",
        requests = 0x00000040,
        duration = 4,
        timer = "immobilize",
    },
    confuse_480 = new.status {
        id = "confuse-480",
        requests = 0x00000080,
        duration = 480,
        timer = "confuse",
    },
    confuse_720 = new.status {
        id = "confuse-720",
        requests = 0x00000080,
        duration = 720,
        timer = "confuse",
    },
    confuse_960 = new.status {
        id = "confuse-960",
        requests = 0x00000080,
        duration = 960,
        timer = "confuse",
    },
    confuse_4 = new.status {
        id = "confuse-4",
        requests = 0x00000080,
        duration = 4,
        timer = "confuse",
    },
    blind_480_past_confuse = new.status {
        id = "blind-480-past-confuse",
        requests = 0x00000020,
        duration = 480,
        timer = "blind",
    },
    blind_720_past_confuse = new.status {
        id = "blind-720-past-confuse",
        requests = 0x00000020,
        duration = 720,
        timer = "blind",
    },
    blind_1200_past_confuse = new.status {
        id = "blind-1200-past-confuse",
        requests = 0x00000020,
        duration = 1200,
        timer = "blind",
    },
    blind_4_past_confuse = new.status {
        id = "blind-4-past-confuse",
        requests = 0x00000020,
        duration = 4,
        timer = "blind",
    },
    immobilize_2_past_confuse = new.status {
        id = "immobilize-2-past-confuse",
        requests = 0x00000040,
        duration = 2,
        timer = "immobilize",
    },
    immobilize_60_past_confuse = new.status {
        id = "immobilize-60-past-confuse",
        requests = 0x00000040,
        duration = 60,
        timer = "immobilize",
    },
    immobilize_120_past_confuse = new.status {
        id = "immobilize-120-past-confuse",
        requests = 0x00000040,
        duration = 120,
        timer = "immobilize",
    },
    immobilize_4_past_confuse = new.status {
        id = "immobilize-4-past-confuse",
        requests = 0x00000040,
        duration = 4,
        timer = "immobilize",
    },
    after_table_1_past_confuse = new.status {
        id = "after-table-1-past-confuse",
        requests = 0x00000000,
        duration = 65535,
        timer = { other = 10 },
    },
    after_table_2_past_confuse = new.status {
        id = "after-table-2-past-confuse",
        requests = 0xFFFF0000,
        duration = 100,
        timer = { other = 0 },
    },
    after_table_3_past_confuse = new.status {
        id = "after-table-3-past-confuse",
        requests = 0x00C8FFFF,
        duration = 0,
        timer = { other = 255 },
    },
    after_table_4_past_confuse = new.status {
        id = "after-table-4-past-confuse",
        requests = 0x00000096,
        duration = 65535,
        timer = { other = 1 },
    },
    blind_480 = new.status {
        id = "blind-480",
        requests = 0x00000020,
        duration = 480,
        timer = "blind",
    },
    blind_720 = new.status {
        id = "blind-720",
        requests = 0x00000020,
        duration = 720,
        timer = "blind",
    },
    blind_1200 = new.status {
        id = "blind-1200",
        requests = 0x00000020,
        duration = 1200,
        timer = "blind",
    },
    blind_4 = new.status {
        id = "blind-4",
        requests = 0x00000020,
        duration = 4,
        timer = "blind",
    },
    immobilize_2_past_blind = new.status {
        id = "immobilize-2-past-blind",
        requests = 0x00000040,
        duration = 2,
        timer = "immobilize",
    },
    immobilize_60_past_blind = new.status {
        id = "immobilize-60-past-blind",
        requests = 0x00000040,
        duration = 60,
        timer = "immobilize",
    },
    immobilize_120_past_blind = new.status {
        id = "immobilize-120-past-blind",
        requests = 0x00000040,
        duration = 120,
        timer = "immobilize",
    },
    immobilize_4_past_blind = new.status {
        id = "immobilize-4-past-blind",
        requests = 0x00000040,
        duration = 4,
        timer = "immobilize",
    },
    after_table_1_past_blind = new.status {
        id = "after-table-1-past-blind",
        requests = 0x00000000,
        duration = 65535,
        timer = { other = 10 },
    },
    after_table_2_past_blind = new.status {
        id = "after-table-2-past-blind",
        requests = 0xFFFF0000,
        duration = 100,
        timer = { other = 0 },
    },
    after_table_3_past_blind = new.status {
        id = "after-table-3-past-blind",
        requests = 0x00C8FFFF,
        duration = 0,
        timer = { other = 255 },
    },
    after_table_4_past_blind = new.status {
        id = "after-table-4-past-blind",
        requests = 0x00000096,
        duration = 65535,
        timer = { other = 1 },
    },
    after_table_5_past_blind = new.status {
        id = "after-table-5-past-blind",
        requests = 0xFFFF0000,
        duration = 1,
        timer = { other = 200 },
    },
    after_table_6_past_blind = new.status {
        id = "after-table-6-past-blind",
        requests = 0x00010BB8,
        duration = 300,
        timer = { other = 184 },
    },
    after_table_7_past_blind = new.status {
        id = "after-table-7-past-blind",
        requests = 0x00000000,
        duration = 0,
        timer = { other = 0 },
    },
    after_table_8_past_blind = new.status {
        id = "after-table-8-past-blind",
        requests = 0x00000000,
        duration = 0,
        timer = { other = 0 },
    },
    immobilize_2 = new.status {
        id = "immobilize-2",
        requests = 0x00000040,
        duration = 2,
        timer = "immobilize",
    },
    immobilize_60 = new.status {
        id = "immobilize-60",
        requests = 0x00000040,
        duration = 60,
        timer = "immobilize",
    },
    immobilize_120 = new.status {
        id = "immobilize-120",
        requests = 0x00000040,
        duration = 120,
        timer = "immobilize",
    },
    immobilize_4 = new.status {
        id = "immobilize-4",
        requests = 0x00000040,
        duration = 4,
        timer = "immobilize",
    },
    after_table_1_past_immobilize = new.status {
        id = "after-table-1-past-immobilize",
        requests = 0x00000000,
        duration = 65535,
        timer = { other = 10 },
    },
    after_table_2_past_immobilize = new.status {
        id = "after-table-2-past-immobilize",
        requests = 0xFFFF0000,
        duration = 100,
        timer = { other = 0 },
    },
    after_table_3_past_immobilize = new.status {
        id = "after-table-3-past-immobilize",
        requests = 0x00C8FFFF,
        duration = 0,
        timer = { other = 255 },
    },
    after_table_4_past_immobilize = new.status {
        id = "after-table-4-past-immobilize",
        requests = 0x00000096,
        duration = 65535,
        timer = { other = 1 },
    },
    after_table_5_past_immobilize = new.status {
        id = "after-table-5-past-immobilize",
        requests = 0xFFFF0000,
        duration = 1,
        timer = { other = 200 },
    },
    after_table_6_past_immobilize = new.status {
        id = "after-table-6-past-immobilize",
        requests = 0x00010BB8,
        duration = 300,
        timer = { other = 184 },
    },
    after_table_7_past_immobilize = new.status {
        id = "after-table-7-past-immobilize",
        requests = 0x00000000,
        duration = 0,
        timer = { other = 0 },
    },
    after_table_8_past_immobilize = new.status {
        id = "after-table-8-past-immobilize",
        requests = 0x00000000,
        duration = 0,
        timer = { other = 0 },
    },
    after_table_9_past_immobilize = new.status {
        id = "after-table-9-past-immobilize",
        requests = 0x00000000,
        duration = 0,
        timer = { other = 0 },
    },
    after_table_10_past_immobilize = new.status {
        id = "after-table-10-past-immobilize",
        requests = 0x00000000,
        duration = 0,
        timer = { other = 0 },
    },
    after_table_11_past_immobilize = new.status {
        id = "after-table-11-past-immobilize",
        requests = 0x00000000,
        duration = 0,
        timer = { other = 0 },
    },
    after_table_12_past_immobilize = new.status {
        id = "after-table-12-past-immobilize",
        requests = 0x00000000,
        duration = 0,
        timer = { other = 0 },
    },
    -- The status rules, EXE4's as read and, where exe4-map.md §18 lists a gap, the nearest the engine can say. Its
    -- NaviCust's HP bug drains a point every NaviStats +0x0E ticks (0x0800C164: the byte is the period, no table),
    -- through a pause; its custom screen's every +0x0F ticks while the side's selection runs (0x0800C194: the
    -- status's bit 0).
    rules = {
        hp_drain = { periods = "stat", stops_while_paused = false },
        custom_drain = { periods = "stat", status = 0x1 },
        -- No per-form tick (the player's update, 0x080EAECC, runs none: a soul's is its own, to read with the souls).
        form_tick = false,
        -- The mercy flash hides the navi while its timer's bit 1 is clear (0x08010430).
        flash_hides_on_clear = true,
        -- What a navi's status word reads as before its init made its collision data: EXE5's open-bus value, to
        -- confirm by recording (exe4-map.md §16).
        missing_collision_status = 0xE55EC002,
        -- Its status block (0x08013A48) is EXE5's order: the flash's timer first, whatever the battle's flags
        -- (0x080134F6), then the slides, a drag and a flinch, which end a paralysis unless a counter just made it.
        reactions = "flash_timer_first",
        -- Its reaction actions are plain (0x08010960, 0x080109FA, 0x08010ABC, 0x08010C16): none marks the action in
        -- use; the flinch and the paralysis keep the height, the paralysis snaps sliding or not and counts no
        -- reaction; the drag counts none.
        reaction_actions = "plain",
        -- A drag takes the flinch's pose (0x08010ABC), and keeps it to its end, to idle (0x08010C16).
        drag = { poses = { otherwise = 1 }, ending = "keeps_pose" },
        -- A hit's NaviCust bug is taken (0x0800D9E8) before the HP bug drains (0x0800C164); no no-charge drive.
        bugs_before_drain = true,
        no_charge_drive = false,
        -- A player's hit drains its side's gauge with the HP and shows only with HP left; at 0 HP the rules are asked
        -- after (0x08013A48: 0x0800AB9E, 0x0800EBC8); every other loss takes the HP alone (0x0800AB92).
        hp_loss = "hit_drains_gauge",
        -- Every console hears a navi's hit as `hit` (0x6B), a navi in auto battle's (NaviStats +0x26 = 1) as
        -- `auto_battle_hit` (0x6D): its hit intake, 0x08013A8C.
        hit_sound = "by_navi",
        -- A barrier (0x08012DF8) runs whenever its navi does (its timer counts in the intro's pause); wind takes it
        -- away at once (0x08012E1E), the hit absorbed all the same.
        barrier = { stops_while_paused = false, wind = "takes_away" },
        -- EXE4's emotions (0x0800F49C, in its order: the first that holds is the side's): in a soul (NaviStats +0x24,
        -- neither 0 nor 15) the soul's (its face the soul form's); angry (AIData +0x2E, anger's ticks); a mood of 0
        -- (the dark MegaMan's: worn out); Full Synchro (0xFF); a mood under 65 worried; else normal. Its setter
        -- (0x0800F4DE) leaves a mood of 0, and anger's end goes through it (0x0800C608).
        emotion = {
            mood_held = "at_zero",
            anger_end = "through_setter",
            order = {
                { emotion = "soul", when = { { in_form = true } } },
                { emotion = "angry", when = { { angry = true } } },
                { emotion = "worn_out", when = { { mood = 0 } } },
                { emotion = "full_synchro", when = { { mood = 0xFF } } },
                { emotion = "worried", when = { { mood_below = 65 } } },
                { emotion = "normal" },
            },
            -- What the framework does by them: Full Synchro and anger double the next damaging chip (0x0800D54E), Full
            -- Synchro with its aura besides (0x0800D9AE); worn out or in a soul, anger doesn't start (0x0800C56E: the
            -- soul, 0x0800C57E: the emotion 5; the framework's `tired`). The buster knows no emotion.
            roles = { full_synchro = "full_synchro", angry = "angry", worn_out = "tired", soul = "tired" },
            -- A hit's counter byte raises the hitter's mood and wears the receiver's, a counter hit's counting 0xFF
            -- (Full Synchro) and wearing 0x7F (0x08012C10, 0x080131E4). A chip's use leaves Full Synchro at 0x99
            -- (0x0800D568) and plays the boost's sound for it alone (anger's ends silent, 0x0800D57C).
            hit_mood = "hitter_gains",
            full_synchro_spent = 0x99,
            anger_boost_sound = false,
            -- (No `anger_damage`: only 120 ticks stunned ask for anger, 0x0800C540; no damage does.)
        },
        -- (A soul's break by a weakness hit: EXE5's until the souls are read.)
        form_break = "any_form",
        weakness_hit_breaks_form = false,
        -- The weakness mark shows for the damage of the element the navi is weak to (0x080133CE, the table at
        -- 0x08013408), as EXE5's.
        weakness_mark = "weak_element_damage",
        -- A navi runs through a pause until it takes control (0x08010A88 clears its header flag 0x04, and the object
        -- loop skips it while paused from then on); its status block has no pause handler, and runs its action
        -- (0x08013C2A).
        paused_navi = "stops_at_control",
        -- Its status timers count while paused (0x0800AE58 has no pause test): before it takes control, through the
        -- round's first paused ticks.
        timers_while_paused = true,
        -- Its idle (MegaMan's, 0x080EEB38) stands the navi (animation 0) on each tick past its first phase
        -- (0x080EEBAC), the 10 ticks after a reaction's end: a drag's kept pose (`drag`) holds those ticks.
        idle_stands = true,
        -- (No `link_body_damage`: EXE4 has no counterpart of EXE6's `sub_80142B0` or `sub_80142C2`, so a navi's body
        -- hits for its object's zeroed damage, in a link battle too: the lab's WhitWeb webs take none from it.)
    },
}

return status
