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
    damage_word = "paralysis_and_bugs",
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
    -- Cross's or a Beast's own as the navi changes form), and runs through every pause with its sprite still.
    full_synchro_aura = {
        follows_identity = true,
        steps_while_paused = false,
        stops_at_a_pause_in_the_fight = false,
    },
}
