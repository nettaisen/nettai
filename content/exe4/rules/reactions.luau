--!strict
-- Reactions: EXE4's push rows (0x0800ACDC: six and none), the slide and drag speeds (0x0801077C, 0x080111E8:
-- EXE5's), its ice push (0x0801335A), its hit test and request clears, and its stance's counter, the nearest the
-- engine can say until read (exe4-map.md §18).
-- Written by tools/exe4/gen_rules.py (bn6battle-verify) from the EXE4 ROM; people own it now.

return {
    -- A push reads the first of bits 2 to 7 of the hits' final modifier (0x0800ACAA): back, forward, a panel back, a
    -- panel forward, up, down, toward the navi's front; an obstacle's the same from its copy of the rows (0x0800B1C0,
    -- 0x0800B218), toward the pusher. A push with the flinch bit is a drag (0x08013858: 0x080138D8), and an obstacle's
    -- push (0x0801393E); whatever started it, a slide and a drag read the push (0x08010294, 0x08010ABC).
    push_reading = {
        reads = "final",
        bits = 6,
        drag_bit = 0x01,
        obstacle_rows = {
            { dx = 1, dy = 0, panels = 6 },
            { dx = -1, dy = 0, panels = 6 },
            { dx = 1, dy = 0, panels = 1 },
            { dx = -1, dy = 0, panels = 1 },
            { dx = 0, dy = -1, panels = 1 },
            { dx = 0, dy = 1, panels = 1 },
            { dx = 0, dy = 0, panels = 0 },
        },
    },
    -- The hit test (0x08012AFC): no FloatShoe test, no body under a sea (EXE4 has none), a guard breaks to types with
    -- 0x1002 (0x08012B8C) and turns aside what lacks 0x0C004000 (the engine's 0x0C005000, which a type with 0x1000
    -- never reaches), and an untouchable body is tested with an invulnerable one, after its guard (0x08012BDC).
    hit_test = {
        float_shoe_needs_self_bit = false,
        bubbled_as_submerged = false,
        elec_reaches_submerged = false,
        guard_breaks_to = 0x1002,
        guard_before_untouchable = true,
    },
    -- A move's direction in the collision record (0x0800AF90, from 0x08012D9A): up, down, left or right whatever the
    -- side, across before up and down, no other.
    move_direction = "absolute",
    -- An obstacle's slide keeps no bounds (0x0801145C, EXE5's code).
    obstacle_slide_bounds = false,
    -- What the ends of a navi's actions clear of its requests besides the attacks' six: an attack's (0x0800CA28,
    -- 0x0800C9FC: 0x3F) nothing more; a paralysis's, a flinch's and a drag's (0x08010A74, 0x080109E6, 0x08010C28:
    -- 0x43F) the anti-sword trigger too (0x400, which AntiSwrd's trap sets: 0x0802309C).
    request_clears = {
        attack = {},
        paralysis = { "anti_sword_triggered" },
        flinch = { "anti_sword_triggered" },
        drag = { "anti_sword_triggered" },
    },
    push = {
        { dx = 1, dy = 0, panels = 6 },
        { dx = -1, dy = 0, panels = 6 },
        { dx = 1, dy = 0, panels = 1 },
        { dx = -1, dy = 0, panels = 1 },
        { dx = 0, dy = -1, panels = 1 },
        { dx = 0, dy = 1, panels = 1 },
        { dx = 0, dy = 0, panels = 0 },
        { dx = 0, dy = 0, panels = 0 },
        { dx = 0, dy = 0, panels = 0 },
        { dx = 0, dy = 0, panels = 0 },
    },
    -- A move's end on ice pushes a body (0x0801335A): a push bit by its side and the direction of the move (the table
    -- at 0x080133B4: none, up, down, left, right) ORed into its collision's final modifier, which the intake reads
    -- after (a slide by the push's row, or a drag with a hit's flinch bit).
    ice = { push = { { 0x00, 0x40, 0x80, 0x20, 0x10 }, { 0x00, 0x40, 0x80, 0x10, 0x20 } } },
    -- (EXE4 has no bubbled reaction: six framework states.)
    bubble_bob = { 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0 },
    slide_speed = { x = 0xA_0000, y = 0x8_0000 },
    -- What a navi wears restarts by reloading its animation (0x080CC61A, EXE5's code).
    overlay_restart = "reload",
    -- (A stance's counter: EXE4's to read.)
    stance_counter = "next_tick",
}
