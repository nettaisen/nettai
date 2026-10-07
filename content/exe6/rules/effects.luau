--!strict
-- EXE6's shared battle effects where games' touch the simulation differently (docs/design/exe5-map.md §15.3 items
-- 15 and 16), the arena's: its camera shake (`camera_doShakeEffect_80301e8`) has two channels (the primary and the
-- secondary, `sub_80302B6`), each console's drawing its jitter from its own RNG1; its hit spark steps its sprite
-- once as it starts (`sub_80E0864`); setting an object's collision types again (`sub_801A082`) sets what it is and
-- what it hits, marked as made while dimmed when the battle is; its damage words' flag bits (`sub_8019F44`) are
-- 0x4000 the paralysis with a flinch, then 0x2000 bug code 0xF8, else 0x1000 bug code 0xF7; its screen palette flash (effect object 0x0A, `sub_80E10C0`,
-- `sub_80E114C`) keeps going through a pause and dimming with its mode's bit 1, else a pause holds it and dimming
-- does unless the variant's bit is set.

return {
    shake = "console_rng",
    spark_steps_at_start = true,
    retype = "is_and_hits",
    -- Its damage words (`sub_8019F44`): the damage the low 11 bits, doubled with 0x8000; then 0x4000 the paralysis with a
    -- flinch, then 0x2000 bug code 0xF8 and no more, else 0x1000 bug code 0xF7, each code's high byte the target lookup's
    -- row offset (what the lookup left in r1).
    damage_word = {
        damage = 0x7FF,
        flags = {
            { bit = 0x4000, status = "damage_word_paralysis", hit_modifier = 1 },
            { bit = 0x2000, bug = 0xF8, bug_high_row = true, stop = true },
            { bit = 0x1000, bug = 0xF7, bug_high_row = true },
        },
    },
    palette_flash = "mode_runs_through_pause",
    -- Presentation: its flash is a palette transform in slot 20 (`sub_80E10C0`), after the fade system's first record's
    -- (slots 18 and 19: a dimming's), so its white stands over a dimming.
    palette_flash_order = "after_fades",
    -- Its afterimages (`sub_80E33FA`) and form overlays (`sub_80C4530`'s spawner) run while the battle is paused
    -- (their header flag 0x04).
    overlays_run_while_paused = true,
    -- An afterimage that copies its owner (`sub_80E32D8`) takes its NameID and wears what that record's init hook
    -- puts on (a Cross's or a Beast's overlay), taken off as it goes.
    afterimages_wear_overlays = true,
    -- Its sprites' frame load (`sprite_loadAnimationData`, `sub_3006730`) takes the palette offset of the frame's
    -- first part.
    load_sets_part_palette = true,
    -- Its obstacles' action tables have the actors' frozen and bubbled entries at 6 and 7: a kind's own actions
    -- from 8.
    obstacle_actions = "own_from_8",
    -- Its Full Synchro aura (actor object 0x5E, `sub_80C4B18`) takes its navi's identity's animation each tick (a
    -- Cross's or a Beast's own as the navi changes form), and runs through every pause with its sprite still, from
    -- its spawn (`sub_80C4C12` sets the header flag 0x04).
    full_synchro_aura = {
        follows_identity = true,
        steps_while_paused = false,
        stops_at_a_pause_in_the_fight = false,
        spawn_runs_while_paused = true,
    },
    -- A navi's charge glow (effect 8, `sub_80E0F02`) comes with it.
    charge_glow = "with_navi",
    -- The held button raises a hold request and charges to full (`sub_8012FC8`, `sub_8012EBC`): A then B, pressing the
    -- other switching; L and R turn the navi, or with a full gauge ask for the custom screen; B then back (by facing).
    charge = "hold_flags",
    -- A held direction steps by the first of up, down, right, left held (`sub_800FA54`); confused, up and down swap,
    -- right and left (`byte_800FAA4`); the idle starts the step whether or not it can go (`sub_80F0354`).
    steps = {
        keys = { "up", "down", "right", "left" },
        confused = { up = "down", down = "up", left = "right", right = "left" },
        idle_checks_target = false,
    },
    -- A screen fade toward clear ends on the step whose level reaches its target (`sub_8006366`).
    fade_clear = "at_target",
    -- A banner (`sub_801CE28`) slides in for 5 ticks, holds 0x30, slides out for 5; one that holds until let go has
    -- its timer stop at 5, and let go (`sub_801E780`) holds three ticks more. It bounces as its hold starts and ends.
    banner = { slide_in = 5, hold = 0x30, slide_out = 5, release = "holds_three_more", bounces = true },
    -- Each console's RNG1 advances once a frame after the battle's (`main_`'s `GetRNG1`).
    rng1_per_frame = true,
    -- An object's destroy (`object_genericDestroy`) releases what it holds and frees its collision as it is.
    destroy = "frees",
}
