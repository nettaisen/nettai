--!strict
-- The test content's reactions: made-up tables for the engine's tests, and
-- the choices a game makes where games differ (the test content plays as
-- EXE6 does).

local NONE = { dx = 0, dy = 0, panels = 0 }

-- The bubble's bob by its timer: up three pixels and back, twice, then
-- down and back, twice.
local bob = {}
for i = 0, 31 do
    local step = ({ 0, 1, 2, 3, 3, 2, 1, 0 })[i % 8 + 1]
    bob[i + 1] = if i < 16 then step else -step
end

return {
    -- A push reads the first of bits 2 to 5 of the hits' final modifier,
    -- toward the navi's front; the 0x80 bit picks the last five rows. An
    -- obstacle's push reads the same bits from its own four rows, toward
    -- the pusher. A push with 0x40 is a drag (an obstacle's push).
    push_reading = {
        reads = "final",
        bits = 4,
        drag_bit = 0x40,
        shift = { bit = 0x80, rows = 5 },
        obstacle_rows = {
            { dx = -1, dy = 0, panels = 6 },
            { dx = 1, dy = 0, panels = 6 },
            { dx = -1, dy = 0, panels = 1 },
            { dx = 1, dy = 0, panels = 1 },
        },
    },
    -- A FloatShoe body meets only collision types with the 0x80 self bit;
    -- the flag 0x80000000 is a bubble, on which an elec hit counts once
    -- more; a guard breaks to types with 0x2.
    hit_test = {
        float_shoe_needs_self_bit = true,
        bubbled_as_submerged = false,
        elec_reaches_submerged = false,
        guard_breaks_to = 0x0002,
        guard_before_untouchable = false,
        guard_marks_direction = true,
    },
    -- A pulled obstacle stays out of the puller's area.
    obstacle_slide_bounds = true,
    -- Pushes by hit-modifier bit, then the five with the 0x80 bit.
    push = {
        { dx = 1, dy = 0, panels = 6 },
        { dx = -1, dy = 0, panels = 6 },
        { dx = 1, dy = 0, panels = 1 },
        { dx = -1, dy = 0, panels = 1 },
        NONE,
        { dx = 0, dy = -1, panels = 1 },
        { dx = 0, dy = 1, panels = 1 },
        { dx = 1, dy = 0, panels = 2 },
        { dx = -1, dy = 0, panels = 2 },
        NONE,
    },
    -- A move's end on ice slides by the direction the navi last moved
    -- (none, up, down, back, forward, other).
    slide_rows = {
        NONE,
        { dx = 0, dy = -1, panels = 1 },
        { dx = 0, dy = 1, panels = 1 },
        { dx = -1, dy = 0, panels = 1 },
        { dx = 1, dy = 0, panels = 1 },
        NONE,
    },
    bubble_bob = bob,
    -- A slide and a drag go 10 pixels a tick across and 6 in depth.
    slide_speed = { x = 0xA_0000, y = 0x6_0000 },
    -- What a navi wears restarts with a step at once.
    overlay_restart = "step",
    -- The counter a stance's caught hit starts runs from the next tick.
    stance_counter = "next_tick",
    trap_counter = "at_once",
    dead_player = "kept",
    attack_end_lockout = "by_kind",
    -- A move's direction: up, down, back or forward by the side.
    move_direction = "by_side",
    -- What the ends of a navi's actions clear of its requests besides the attacks': both games' (the mode-9 A
    -- press and the Chaos Unison releases at an attack's end; the anti-sword trigger at a flinch's and a drag's).
    request_clears = {
        attack = { "mode9_a", "rules_release" },
        paralysis = { "mode9_a" },
        flinch = { "anti_sword_triggered", "mode9_a" },
        drag = { "anti_sword_triggered", "mode9_a" },
    },
}
