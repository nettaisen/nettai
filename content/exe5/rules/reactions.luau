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
    push_reading = "exe5",
    -- The hit test (0x0801691C): a bubbled body counts as submerged, elec reaching either; a FloatShoe body is hit by
    -- all (EXE5's collision types have no 0x80 self bit); a guard breaks to 0x1002 and turns aside what lacks
    -- 0x0C004000.
    hit_test = "exe5",
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
    ice = {
        { dx = 0, dy = 0, panels = 0 },
        { dx = 0, dy = -1, panels = 1 },
        { dx = 0, dy = 1, panels = 1 },
        { dx = -1, dy = 0, panels = 1 },
        { dx = 1, dy = 0, panels = 1 },
        { dx = 0, dy = 0, panels = 0 },
    },
    bubble_bob = { 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0 },
    slide_speed = { x = 0xA_0000, y = 0x8_0000 },
    overlay_restart = "reload",
    stance_counter = "at_once",
}
