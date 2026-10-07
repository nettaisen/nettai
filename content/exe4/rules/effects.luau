--!strict
-- EXE4's shared battle effects where they touch the simulation otherwise than EXE6's (docs/design/exe4-map.md §15
-- effects), the arena's: its camera shake (0x08025FE0) draws its jitter from the battle's RNG2 and holds while the
-- battle is paused without dimming (EXE5's, and EXE3's 0x08022AE4 before it); its hit spark (EXE5's 0x080E0870, the
-- same code) doesn't step its sprite as it starts; setting an object's collision types again (0x08012ED0) sets what
-- it is alone (EXE5's); its screen palette flash (0x080E2A34) holds as EXE5's. Its damage words (0x08012860) are
-- its own: the damage the low 14 bits, doubled with 0x8000, and 0x4000 paralysis (status byte 0x12) without a
-- flinch, nothing else.

return {
    shake = "battle_rng",
    spark_steps_at_start = false,
    retype = "is_alone",
    damage_word = {
        damage = 0x3FFF,
        flags = {
            { bit = 0x4000, status = "damage_word_paralysis", hit_modifier = 0, stop = true },
        },
    },
    palette_flash = "pause_holds",
    -- Presentation: its flash is a palette transform in slot 9, before the fades' (EXE4's fade slots to confirm:
    -- exe4-map.md §16).
    palette_flash_order = "before_fades",
    -- Its afterimages and form overlays don't run while paused (EXE5's code).
    overlays_run_while_paused = false,
    afterimages_wear_overlays = false,
    -- Its sprites' frame load (IWRAM 0x0300632C) is EXE5's code.
    load_sets_part_palette = false,
    -- Its obstacles' action tables have no frozen or bubbled entries: a kind's own actions from 6 (the player's
    -- table, 0x080EAEFC: six framework states; an obstacle's to confirm, exe4-map.md §16).
    obstacle_actions = "own_from_6",
    -- Its Full Synchro aura (0x080CD180) is EXE5's; its spawn (0x080CD276) leaves it to wait while the battle is paused.
    full_synchro_aura = {
        follows_identity = false,
        steps_while_paused = true,
        stops_at_a_pause_in_the_fight = true,
        spawn_runs_while_paused = false,
    },
    -- A navi's init spawns no charge glow (0x0801079C): its charge brings its own (0x0800BD88, effect 5 0x080E215C,
    -- which goes with the charge; exe4-map.md §18 item 39).
    charge_glow = "with_charge",
    -- Its buttons charge by the button held (0x0800BDE0, 0x0800BBA4, 0x0800BB50): B first, its count going on past
    -- full; the buster on B's release; B then Left (whichever way the navi faces) for the B+Left special; L and R
    -- are no navi's.
    charge = "per_button",
    -- A held direction steps by the first of right, left, up, down held (0x0800B4B0); confused, up and down are
    -- swapped, and right and left (0x0800B550: 1, 0, 3, 2 by the keys' index, down 0 to right 3, as EXE6's); the idle
    -- starts a step only toward a panel the navi may step to (0x080EEC82). The move bug (NaviStats +0x0D, the stat
    -- `move_bug`: ./navicust's) confuses the navi for 720 ticks as the round starts at 0xFF (0x0800D8B0); else, with
    -- no key held (or sliding), its keys step it (0x0800B4DA: 0x10 right, 0x20 left).
    steps = {
        keys = { "right", "left", "up", "down" },
        confused = { up = "down", down = "up", left = "right", right = "left" },
        idle_checks_target = true,
        bug = { stat = "move_bug", confused = 720 },
    },
    -- A screen fade toward clear ends on the step whose level would go under its target (0x08005BDE, which doesn't
    -- keep that level): a step after EXE6's, so its intro's fade from white takes 18 steps.
    fade_clear = "past_target",
    -- A banner (0x08014904, its steps from 0x08014994) slides in for 9 ticks, holds 30, slides out for 9; one that
    -- holds until let go (its table's state 2) doesn't count its hold, and let go (0x0801616C) slides out at once. Its
    -- hold sets no squash (0x080149CE): it doesn't bounce.
    banner = { slide_in = 9, hold = 30, slide_out = 9, release = "slides_out", bounces = false },
    -- Its main loop (0x080002B0) draws no RNG1 a frame: RNG1 moves only where the battle draws it.
    rng1_per_frame = false,
    -- The chip icons over the local navi (0x08014860, 0x08015000): at its identity's `chip_icons_at` from its place on
    -- the screen (0x0800B9E4 by navi number, which 0x08015B24 keeps), each chip it holds from the next one on by its own
    -- icon (the chip record's +0x20), each next one two pixels up and two left: sprites of priority 1 in the field's layer,
    -- the first in the depth bucket of the icons' count and each next one a bucket lower. EXE4 has no attach points.
    chip_icons = "navi_offset",
    -- The other player's console names a chip a player used (0x0800C7A8 to 0x080164B4) as a banner of the second block
    -- (0x02037CF0) that shows without sliding (its table's state 3, 0x08016508): drawn the tick it is laid out, through
    -- its hold of 30 (0x080149CE) and the tick it lets go of its sprites (0x080149E6), 32 frames (the count includes the
    -- tick it starts on, as EXE6's 0x3C does).
    used_chip_ticks = 33,
    -- Its emotion window has no bug check: a bugged navi's face doesn't flicker, and RNG1 doesn't move for it.
    bug_flicker = false,
    -- A pool's objects are drawn by slot, from the first (0x08003BA0, 0x08003ED4, 0x08004180 walk the pools' records),
    -- not in update order: of two parts in one depth bucket the higher slot's is in front.
    draw_order = "slots",
}
