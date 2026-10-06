--!strict
-- The test content's status rules (the rules' `status` section): the HP
-- bug's made-up drain periods by bug level, and the choices a game makes
-- where games differ (the test content plays as EXE6 does). Its statuses
-- are rules/status.luau's.

return {
    hp_bug_periods = { 0, 60, 50, 40, 30, 20, 10, 5 },
    -- The per-form tick runs (the Fire chips' charge, a form's height).
    form_tick = true,
    -- The mercy flash hides the navi while its timer's bit 1 is set.
    flash_hides_on_clear = false,
    -- A navi without collision data reads as blind (the word's blind bit,
    -- as EXE6's open-bus value has it).
    missing_collision_status = 0xE3A02004,
    -- A drag, the slides and a flinch, then the mercy flash's timer.
    reactions = "flash_timer_last",
    -- The HP bug drains before a hit's NaviCust bug is taken, whose drain
    -- argument adds; no no-charge drive; a loss of HP takes the HP alone.
    bugs_before_drain = false,
    drain_bug_flags = false,
    no_charge_drive = false,
    hp_loss = "hp_alone",
    -- Worn out (exhausted, or a mood of 0) before anger, then held tired, Full
    -- Synchro, else normal; a navi held tired or exhausted holds its mood, and
    -- anger's end puts it at 0x80.
    emotion = {
        mood_held = "tired_or_exhausted",
        anger_end = "resets_mood",
        plain_in_battle_mode_1 = false,
        normal_in_a_form = false,
        anger_before_worn_out = false,
        tired_and_exhausted = true,
    },
    -- The weakness request breaks a Cross or a Beast to what it breaks to.
    form_break = "marked_forms",
    weakness_hit_breaks_form = true,
    -- The weakness mark shows over the last hit's multiplier, on a tick with
    -- damage.
    weakness_mark = "last_hit_multiplier",
}
