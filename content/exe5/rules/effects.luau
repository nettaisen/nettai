--!strict
-- EXE5's shared battle effects where they touch the simulation otherwise than EXE6's (docs/design/exe5-map.md §15.3
-- items 15 and 16), the arena's: its camera shake (0x08030D78) has one channel and draws its jitter from the
-- battle's RNG2, twice a shaking tick, holding while the battle is paused without dimming (EXE6's draws from each
-- console's RNG1); its hit spark (0x080E0870) doesn't step its sprite as it starts, so it lasts a tick longer than
-- EXE6's (`sub_80E0864`); setting an object's collision types again (0x08016B9E, EXE6's `sub_801A082`) sets what it
-- is alone, never marked as made while dimmed (its store of what it hits goes to a BIOS address). (Its obstacles'
-- reactions have the step for ColonelSoul's army, the rules' `obstacle_reaction`: objects/soldier.) Its screen
-- palette flash (effect object 0x0A, 0x080E1030) holds through a pause whatever its
-- mode, and through dimming only with no mode (both variants: EXE6's variant 1 tests the pause bit for dimming too).
-- Its damage words' flag bits (0x080165EC, EXE6's `sub_8019F44`) are its own: 0x4000 paralyzes without a flinch,
-- 0x2000 confuses, 0x1000 blinds, 0x800 gives bug code 0x18 (ColonelSoul's soldiers strike with 0x4000).

return {
    shake = "battle_rng",
    spark_steps_at_start = false,
    retype = "is_alone",
    -- Its damage words (0x080165EC): the damage the low 11 bits, doubled with 0x8000; then 0x4000 the paralysis (status
    -- byte 0x10) without a flinch and no more, else 0x2000 the confusion (0x20) and no more, else 0x1000 the blindness
    -- (0x30); then 0x800 bug code 0x18 with high byte 0x11.
    damage_word = {
        damage = 0x7FF,
        flags = {
            { bit = 0x4000, status = "damage_word_paralysis", hit_modifier = 0, stop = true },
            { bit = 0x2000, status = "damage_word_confusion", stop = true },
            { bit = 0x1000, status = "damage_word_blindness" },
            { bit = 0x800, bug = 0x1118 },
        },
    },
    palette_flash = "pause_holds",
    -- Presentation: its flash is a palette transform in slot 9 (0x080E104C), before the fade system's (slots 18 on), so
    -- a dimming darkens the white it put in the stage's palettes: a flash under a dimming is white a quarter down.
    palette_flash_order = "before_fades",
    -- Its afterimages (0x080E35F4) and form overlays don't run while paused (no header flag 0x04).
    overlays_run_while_paused = false,
    -- Its afterimage (0x080E3550) is a sprite alone: its spawners give it the owner's battle sprite (0x0800DA72) and it
    -- takes no NameID, so an afterimage of a navi that wears something (KnightMan's ball, NumberMan's face) wears none.
    afterimages_wear_overlays = false,
    -- Its sprites' frame load (0x03006898) leaves the palette offset of a frame's first part to the next step
    -- (0x03006948): its hit spark, which doesn't step as it starts, shows its first frame in palette 0.
    load_sets_part_palette = false,
    -- Its obstacles' action tables have no frozen or bubbled entries: a kind's own actions from 6 (EXE6's from 8).
    obstacle_actions = "own_from_6",
    -- Its Full Synchro aura (actor object 0x5E, 0x080C45E0) keeps the animation it started with (0x080C4648), steps
    -- its sprite paused or not, and once the fight is on stops running while the battle is paused (its header flag
    -- goes). It is hidden while its navi is under the sea's surface too (the reactions' `hit_test`). Its spawn
    -- (0x080C46E2) sets no header flag: it waits while the battle is paused.
    full_synchro_aura = {
        follows_identity = false,
        steps_while_paused = true,
        stops_at_a_pause_in_the_fight = true,
        spawn_runs_while_paused = false,
    },
    -- A navi's charge glow (effect 8) comes with it, as EXE6's.
    charge_glow = "with_navi",
    -- Its buttons charge by hold requests, as EXE6's.
    charge = "hold_flags",
    -- Its steps are EXE6's.
    steps = {
        keys = { "up", "down", "right", "left" },
        confused = { up = "down", down = "up", left = "right", right = "left" },
        idle_checks_target = false,
        -- (No move bug of a rules stat: the processing bug, astray steps, is the engine's own.)
    },
    -- A screen fade toward clear ends on the step whose level reaches its target, as EXE6's.
    fade_clear = "at_target",
    -- A banner's steps are EXE6's.
    banner = { slide_in = 5, hold = 0x30, slide_out = 5, release = "holds_three_more", bounces = true },
    -- Each console's RNG1 advances once a frame after the battle's, as EXE6's.
    rng1_per_frame = true,
    -- The chip icons over a navi are EXE6's (its attach point 3).
    chip_icons = "attach_point",
    -- The other player's console names a chip a player used for a second, as EXE6's.
    used_chip_ticks = 0x3C,
    -- The emotion window flickers a bugged navi's face at its checks, an RNG1 draw each (0x08019780, EXE6's).
    bug_flicker = true,
    -- A step leaves its NaviCust panel trail at a chance of its level in 8, an RNG2 draw, as EXE6's.
    panel_trail = "by_chance",
    -- The status visual (effect 6) sits at its navi's sprite attach point 5, hidden off the field, and casts its
    -- sprite's shadow, as EXE6's.
    status_visual = { place = "attach_point" },
    -- A pool's objects are drawn in update order, as EXE6's.
    draw_order = "update_list",
}
