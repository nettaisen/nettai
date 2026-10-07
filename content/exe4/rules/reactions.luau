--!strict
-- Reactions: EXE4's push rows (0x0800ACDC: six and none), the slide and drag speeds (0x0801077C, 0x080111E8:
-- EXE5's), and what EXE4's code does where the engine can say it. Its ice is a push (0x0801335A: the move's
-- direction ORs a push bit in), its hit test, request clears and stance its own: the nearest the engine can
-- say, each a gap (exe4-map.md §18).
-- Written by tools/exe4/gen_rules.py (bn6battle-verify) from the EXE4 ROM; people own it now.

return {
    -- A push reads the first of bits 2 to 7 of the hits' final modifier (0x0800ACAA): back, forward, a panel back, a
    -- panel forward, up, down, toward the navi's front; an obstacle's the same from its copy of the rows (0x0800B1C0,
    -- 0x0800B218), toward the pusher.
    push_reading = {
        reads = "final",
        bits = 6,
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
    -- 0x1002 (0x08012F24) and turns aside what lacks 0x0C004000; an elec hit counts once more on ice, a fire one on
    -- grass (0x08012CF2: elec on ice a gap, the engine's bonus being the sea's).
    hit_test = {
        float_shoe_needs_self_bit = false,
        bubbled_as_submerged = false,
        elec_reaches_submerged = false,
        guard_breaks_to = 0x1002,
        elec_bonus_on_sea = false,
    },
    -- An obstacle's slide keeps no bounds (0x0801145C, EXE5's code).
    obstacle_slide_bounds = false,
    -- An attack's end zeroes the AI data's request halfword (0x0800CA4A): a gap; nothing more cleared meanwhile.
    request_clears = { attack = {}, paralysis = {}, flinch = {}, drag = {} },
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
    -- (EXE4's ice pushes: no ice slide rows.)
    ice = {
        { dx = 0, dy = 0, panels = 0 },
        { dx = 0, dy = 0, panels = 0 },
        { dx = 0, dy = 0, panels = 0 },
        { dx = 0, dy = 0, panels = 0 },
        { dx = 0, dy = 0, panels = 0 },
        { dx = 0, dy = 0, panels = 0 },
    },
    -- (EXE4 has no bubbled reaction: six framework states.)
    bubble_bob = { 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0 },
    slide_speed = { x = 0xA_0000, y = 0x8_0000 },
    -- What a navi wears restarts by reloading its animation (0x080CC61A, EXE5's code).
    overlay_restart = "reload",
    -- (A stance's counter: EXE4's to read.)
    stance_counter = "next_tick",
}
