--!strict
-- Statuses (0x0801CEC4, BN6's `off_80209EC`: four groups, paralysis, confusion, blindness and
-- immobilization, by level; their bytes are compat's): what a hit's status does to its target, and the HP
-- bug's drain periods (0x0800E02C, BN6's values). The timers are 2 bytes further into the collision
-- record than BN6's (paralysis at +0x1E). Rows past a group's end read the next groups' rows, or the
-- bytes after the table: named for what they read, as BN6's.
-- Written by tools/bn5/gen_rules.py (bn6battle-verify) from the BN5 ROM; people own it now.

local status = {
    paralyze_90 = define.status {
        id = "paralyze-90",
        requests = 0x00000008,
        duration = 90,
        timer = "paralyze",
    },
    paralyze_120 = define.status {
        id = "paralyze-120",
        requests = 0x00000008,
        duration = 120,
        timer = "paralyze",
    },
    paralyze_150 = define.status {
        id = "paralyze-150",
        requests = 0x00000008,
        duration = 150,
        timer = "paralyze",
    },
    paralyze_4 = define.status {
        id = "paralyze-4",
        requests = 0x00000008,
        duration = 4,
        timer = "paralyze",
    },
    paralyze_300 = define.status {
        id = "paralyze-300",
        requests = 0x00000008,
        duration = 300,
        timer = "paralyze",
    },
    paralyze_600 = define.status {
        id = "paralyze-600",
        requests = 0x00000008,
        duration = 600,
        timer = "paralyze",
    },
    confuse_480_past_paralyze = define.status {
        id = "confuse-480-past-paralyze",
        requests = 0x00000080,
        duration = 480,
        timer = "confuse",
    },
    confuse_720_past_paralyze = define.status {
        id = "confuse-720-past-paralyze",
        requests = 0x00000080,
        duration = 720,
        timer = "confuse",
    },
    confuse_960_past_paralyze = define.status {
        id = "confuse-960-past-paralyze",
        requests = 0x00000080,
        duration = 960,
        timer = "confuse",
    },
    confuse_4_past_paralyze = define.status {
        id = "confuse-4-past-paralyze",
        requests = 0x00000080,
        duration = 4,
        timer = "confuse",
    },
    confuse_300_past_paralyze = define.status {
        id = "confuse-300-past-paralyze",
        requests = 0x00000080,
        duration = 300,
        timer = "confuse",
    },
    confuse_600_past_paralyze = define.status {
        id = "confuse-600-past-paralyze",
        requests = 0x00000080,
        duration = 600,
        timer = "confuse",
    },
    blind_480_past_paralyze = define.status {
        id = "blind-480-past-paralyze",
        requests = 0x00000020,
        duration = 480,
        timer = "blind",
    },
    blind_720_past_paralyze = define.status {
        id = "blind-720-past-paralyze",
        requests = 0x00000020,
        duration = 720,
        timer = "blind",
    },
    blind_1200_past_paralyze = define.status {
        id = "blind-1200-past-paralyze",
        requests = 0x00000020,
        duration = 1200,
        timer = "blind",
    },
    blind_4_past_paralyze = define.status {
        id = "blind-4-past-paralyze",
        requests = 0x00000020,
        duration = 4,
        timer = "blind",
    },
    confuse_480 = define.status {
        id = "confuse-480",
        requests = 0x00000080,
        duration = 480,
        timer = "confuse",
    },
    confuse_720 = define.status {
        id = "confuse-720",
        requests = 0x00000080,
        duration = 720,
        timer = "confuse",
    },
    confuse_960 = define.status {
        id = "confuse-960",
        requests = 0x00000080,
        duration = 960,
        timer = "confuse",
    },
    confuse_4 = define.status {
        id = "confuse-4",
        requests = 0x00000080,
        duration = 4,
        timer = "confuse",
    },
    confuse_300 = define.status {
        id = "confuse-300",
        requests = 0x00000080,
        duration = 300,
        timer = "confuse",
    },
    confuse_600 = define.status {
        id = "confuse-600",
        requests = 0x00000080,
        duration = 600,
        timer = "confuse",
    },
    blind_480_past_confuse = define.status {
        id = "blind-480-past-confuse",
        requests = 0x00000020,
        duration = 480,
        timer = "blind",
    },
    blind_720_past_confuse = define.status {
        id = "blind-720-past-confuse",
        requests = 0x00000020,
        duration = 720,
        timer = "blind",
    },
    blind_1200_past_confuse = define.status {
        id = "blind-1200-past-confuse",
        requests = 0x00000020,
        duration = 1200,
        timer = "blind",
    },
    blind_4_past_confuse = define.status {
        id = "blind-4-past-confuse",
        requests = 0x00000020,
        duration = 4,
        timer = "blind",
    },
    blind_300_past_confuse = define.status {
        id = "blind-300-past-confuse",
        requests = 0x00000020,
        duration = 300,
        timer = "blind",
    },
    blind_600_past_confuse = define.status {
        id = "blind-600-past-confuse",
        requests = 0x00000020,
        duration = 600,
        timer = "blind",
    },
    immobilize_2_past_confuse = define.status {
        id = "immobilize-2-past-confuse",
        requests = 0x00000040,
        duration = 2,
        timer = "immobilize",
    },
    immobilize_60_past_confuse = define.status {
        id = "immobilize-60-past-confuse",
        requests = 0x00000040,
        duration = 60,
        timer = "immobilize",
    },
    immobilize_120_past_confuse = define.status {
        id = "immobilize-120-past-confuse",
        requests = 0x00000040,
        duration = 120,
        timer = "immobilize",
    },
    immobilize_4_past_confuse = define.status {
        id = "immobilize-4-past-confuse",
        requests = 0x00000040,
        duration = 4,
        timer = "immobilize",
    },
    blind_480 = define.status {
        id = "blind-480",
        requests = 0x00000020,
        duration = 480,
        timer = "blind",
    },
    blind_720 = define.status {
        id = "blind-720",
        requests = 0x00000020,
        duration = 720,
        timer = "blind",
    },
    blind_1200 = define.status {
        id = "blind-1200",
        requests = 0x00000020,
        duration = 1200,
        timer = "blind",
    },
    blind_4 = define.status {
        id = "blind-4",
        requests = 0x00000020,
        duration = 4,
        timer = "blind",
    },
    blind_300 = define.status {
        id = "blind-300",
        requests = 0x00000020,
        duration = 300,
        timer = "blind",
    },
    blind_600 = define.status {
        id = "blind-600",
        requests = 0x00000020,
        duration = 600,
        timer = "blind",
    },
    immobilize_2_past_blind = define.status {
        id = "immobilize-2-past-blind",
        requests = 0x00000040,
        duration = 2,
        timer = "immobilize",
    },
    immobilize_60_past_blind = define.status {
        id = "immobilize-60-past-blind",
        requests = 0x00000040,
        duration = 60,
        timer = "immobilize",
    },
    immobilize_120_past_blind = define.status {
        id = "immobilize-120-past-blind",
        requests = 0x00000040,
        duration = 120,
        timer = "immobilize",
    },
    immobilize_4_past_blind = define.status {
        id = "immobilize-4-past-blind",
        requests = 0x00000040,
        duration = 4,
        timer = "immobilize",
    },
    immobilize_300_past_blind = define.status {
        id = "immobilize-300-past-blind",
        requests = 0x00000040,
        duration = 300,
        timer = "immobilize",
    },
    immobilize_600_past_blind = define.status {
        id = "immobilize-600-past-blind",
        requests = 0x00000040,
        duration = 600,
        timer = "immobilize",
    },
    after_table_1_past_blind = define.status {
        id = "after-table-1-past-blind",
        requests = 0x00000000,
        duration = 65535,
        timer = "collision_panel",
    },
    after_table_2_past_blind = define.status {
        id = "after-table-2-past-blind",
        requests = 0xffff0000,
        duration = 30,
        timer = { other = 0 },
    },
    after_table_3_past_blind = define.status {
        id = "after-table-3-past-blind",
        requests = 0x0032ffff,
        duration = 0,
        timer = { other = 255 },
    },
    after_table_4_past_blind = define.status {
        id = "after-table-4-past-blind",
        requests = 0x00000050,
        duration = 65535,
        timer = { other = 100 },
    },
    immobilize_2 = define.status {
        id = "immobilize-2",
        requests = 0x00000040,
        duration = 2,
        timer = "immobilize",
    },
    immobilize_60 = define.status {
        id = "immobilize-60",
        requests = 0x00000040,
        duration = 60,
        timer = "immobilize",
    },
    immobilize_120 = define.status {
        id = "immobilize-120",
        requests = 0x00000040,
        duration = 120,
        timer = "immobilize",
    },
    immobilize_4 = define.status {
        id = "immobilize-4",
        requests = 0x00000040,
        duration = 4,
        timer = "immobilize",
    },
    immobilize_300 = define.status {
        id = "immobilize-300",
        requests = 0x00000040,
        duration = 300,
        timer = "immobilize",
    },
    immobilize_600 = define.status {
        id = "immobilize-600",
        requests = 0x00000040,
        duration = 600,
        timer = "immobilize",
    },
    after_table_1_past_immobilize = define.status {
        id = "after-table-1-past-immobilize",
        requests = 0x00000000,
        duration = 65535,
        timer = "collision_panel",
    },
    after_table_2_past_immobilize = define.status {
        id = "after-table-2-past-immobilize",
        requests = 0xffff0000,
        duration = 30,
        timer = { other = 0 },
    },
    after_table_3_past_immobilize = define.status {
        id = "after-table-3-past-immobilize",
        requests = 0x0032ffff,
        duration = 0,
        timer = { other = 255 },
    },
    after_table_4_past_immobilize = define.status {
        id = "after-table-4-past-immobilize",
        requests = 0x00000050,
        duration = 65535,
        timer = { other = 100 },
    },
    after_table_5_past_immobilize = define.status {
        id = "after-table-5-past-immobilize",
        requests = 0xffff0000,
        duration = 150,
        timer = { other = 0 },
    },
    after_table_6_past_immobilize = define.status {
        id = "after-table-6-past-immobilize",
        requests = 0x00c8ffff,
        duration = 0,
        timer = { other = 255 },
    },
    after_table_7_past_immobilize = define.status {
        id = "after-table-7-past-immobilize",
        requests = 0x00000001,
        duration = 65535,
        timer = { other = 1 },
    },
    after_table_8_past_immobilize = define.status {
        id = "after-table-8-past-immobilize",
        requests = 0x0bb800c8,
        duration = 200,
        timer = { other = 0 },
    },
    after_table_9_past_immobilize = define.status {
        id = "after-table-9-past-immobilize",
        requests = 0x00000000,
        duration = 0,
        timer = { other = 0 },
    },
    after_table_10_past_immobilize = define.status {
        id = "after-table-10-past-immobilize",
        requests = 0x00000000,
        duration = 0,
        timer = { other = 0 },
    },
    -- BN5's per-form tick table (0x080EB1E8) runs none of BN6's `sub_80F0608` (the Fire charge, a form's
    -- height): MegaMan's (0x080F04CE) steps GyroSoul's overlay (rules/souls). Its hit intake (0x080178EC) takes a
    -- hit's NaviCust bug (0x0801103E) before the HP bug drains (0x0800DFEC), and a drain bug's argument goes by its
    -- flags (bit 4 adds, bit 5 subtracts, else it raises the level).
    rules = {
        hp_bug_periods = { 0, 40, 35, 30, 25, 20, 15, 10 },
        form_tick = false,
        -- The mercy flash hides the navi while its timer's bit 1 is clear (0x080137B6; BN6's while it is set).
        flash_hides_on_clear = true,
        -- A navi's status word read before its init made its collision data (through the null pointer, from the
        -- BIOS: a round's first tick, the other side's first barrier's visual asking whether the local navi is blind,
        -- 0x0800CDF4): an interrupt's opcode (read on the console at 0x0800CE18), without BN6's blind bit.
        missing_collision_status = 0xE55EC002,
        -- Its status block (0x08017CC8 on) runs the flash's timer first, whatever the battle's flags (0x080173C4),
        -- then the slides, a drag and a flinch, which end a paralysis unless a counter just made it.
        reactions = "bn5",
        bugs_before_drain = true,
        drain_bug_flags = true,
        -- DarkInvs's no-charge drive: its ticks count down at the intake's end (0x0800DBE0) and their end asks for the
        -- stun strike (action 0x49, the drive's end: chips/darkinvs); the idle hands the step to the systems (the
        -- computer-navi AI, 0x080F03E4); the last 180 ticks flicker gray (0x080136E0).
        no_charge_drive = true,
        -- BN5's emotions (0x08012740): a soul first, then anger, a mood of 0 (a dark MegaMan's), Full Synchro, a mood
        -- under 65 worried; its mood setter (0x080127D6) leaves a mood of 0; its anger tick (0x08011A14) passes over
        -- AI index 23.
        emotions = "bn5",
        -- The weakness request (a dark chip used in a soul, 0x08010070) breaks any form to the base form (0x080122C8).
        form_break = "bn5",
    },
}

return status
