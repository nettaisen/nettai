--!strict
-- Reactions: push slides by hit-modifier bit (0x0800CA24), ice slides by collision direction (0x0800C988),
-- and the bubble's bob.
-- EXE5's push takes the first bit of collision +0x18 (bits 2 to 5: the unflipped hitters' modifiers), else
-- that of +0x19 (the flipped hitters', 0x08016AA6) with the direction reversed (0x0800C9D8, EXE6's
-- `sub_800E548` otherwise): five rows, and none for EXE6's 0x80 flag (rows 5 to 9, zero here). EXE5 has no bubbled reaction
-- (EXE6's action 7 is absent): its bob is zero. A slide and a drag go 8 pixels a tick in depth where EXE6's
-- go 6 (0x0801361E, 0x080143A8). What a navi wears restarts after an animation change, a flinch or a drag by
-- reloading its animation at its next step (0x080C374E), where EXE6's steps it at once. The counter a stance's caught
-- hit starts (ShadowSoul's B+Back) runs its first step at once (0x0800E340), where EXE6's waits for the next tick.
-- Written by tools/exe5/gen_rules.py (bn6battle-verify) from the EXE5 ROM; people own it now.

return {
    -- A push reads the first of bits 2 to 5 of the unflipped hitters' modifier, else the flipped ones' with the
    -- direction reversed (0x0800C9D8); an obstacle's the same from its own rows (0x0800D4B0, the table at 0x0800D53B:
    -- EXE6's four and a fifth of nothing). A push with 0x40 is a drag (0x08017B44, EXE6's `sub_801AEB0`).
    push_reading = {
        reads = "by_hitter_flip",
        bits = 4,
        drag_bit = 0x40,
        obstacle_rows = {
            { dx = -1, dy = 0, panels = 6 },
            { dx = 1, dy = 0, panels = 6 },
            { dx = -1, dy = 0, panels = 1 },
            { dx = 1, dy = 0, panels = 1 },
            { dx = 0, dy = 0, panels = 0 },
        },
    },
    -- The hit test (0x0801691C), where games differ: a FloatShoe body is hit by all (EXE5's collision types have no
    -- 0x80 self bit); the flag 0x80000000 is a body under the sea's surface, tested as a submerged one, and elec
    -- reaches either; a guard breaks to types with 0x1002 (so its own turn-aside mask, 0x0C004000, is the engine's
    -- 0x0C005000 here); an elec hit counts once more on a body standing on a sea panel (0x08016AF6), as fire's on
    -- grass.
    hit_test = {
        float_shoe_needs_self_bit = false,
        bubbled_as_submerged = true,
        elec_reaches_submerged = true,
        guard_breaks_to = 0x1002,
        elec_bonus_on_sea = true,
    },
    -- A move's direction in the collision record (0x0800CC92, EXE6's `sub_800E994`): up, down, back or forward by
    -- the side, other for a move of two panels or more right or down.
    move_direction = "by_side",
    -- An obstacle's slide keeps no bounds: it goes anywhere open (0x08014894).
    obstacle_slide_bounds = false,
    push = {
        { dx = 1, dy = 0, panels = 6 },
        { dx = -1, dy = 0, panels = 6 },
        { dx = 1, dy = 0, panels = 1 },
        { dx = -1, dy = 0, panels = 1 },
        { dx = 0, dy = 0, panels = 0 },
        { dx = 0, dy = 0, panels = 0 },
        { dx = 0, dy = 0, panels = 0 },
        { dx = 0, dy = 0, panels = 0 },
        { dx = 0, dy = 0, panels = 0 },
        { dx = 0, dy = 0, panels = 0 },
    },
    -- A move's end on ice slides a body (0x080171F0), by the direction of the move (0x0800C988): none, up, down,
    -- back, forward, other.
    ice = {
        slide = {
            { dx = 0, dy = 0, panels = 0 },
            { dx = 0, dy = -1, panels = 1 },
            { dx = 0, dy = 1, panels = 1 },
            { dx = -1, dy = 0, panels = 1 },
            { dx = 1, dy = 0, panels = 1 },
            { dx = 0, dy = 0, panels = 0 },
        },
    },
    bubble_bob = { 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0 },
    slide_speed = { x = 0xA_0000, y = 0x8_0000 },
    overlay_restart = "reload",
    stance_counter = "at_once",
    -- What the ends of a navi's actions clear of its requests besides the attacks' six: the Chaos Unison releases,
    -- at an attack's end (0x0800F2DA: 0x1803F); at a paralysis's, a flinch's and a drag's (0x080142A6, 0x080141D0,
    -- 0x080144E0: 0x1843F) the anti-sword trigger too. (A navi's own requests, GyroMan's take-off and landing,
    -- are his idle's to clear.)
    request_clears = {
        attack = { "rules_release" },
        paralysis = { "anti_sword_triggered", "rules_release" },
        flinch = { "anti_sword_triggered", "rules_release" },
        drag = { "anti_sword_triggered", "rules_release" },
    },
}
