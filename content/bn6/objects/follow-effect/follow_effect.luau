--!strict
-- A follow effect (the original's effect object #0x31, `sub_80E3D68`): the
-- shared follow effect (@common/follow-effect), BN6's kind: shown as its
-- owner is, and its looks (the original's Param1, `off_80E3E10`):
--
-- - "none" (looks 0 to 2): nothing more;
-- - "slide" (3): slides out beside its owner until the owner's related1 is
--   gone, then plays out (with BN6's sound);
-- - "flash" (4, DeltaRay's burst): shows for its timer (60 ticks, which its
--   spawner sets again), switching to animation 1 below 55;
-- - "while_in_action" (5): lasts while its owner stays in the action it was
--   spawned in;
-- - "riding" (6, 7: ElecMan's glow in his DElecSwd): the same, riding on
--   its owner;
-- - "timed" (8): the same, 30 ticks at most.
--
-- The original's looks 5 to 8 name the owner's action by number (0xB,
-- 0xD, 0xA, 0xB), each the action that spawns them; here the effect keeps
-- the action its owner was in when it came. Its flip and a palette to add
-- are its spawner's (Param2, Param3). See docs/engine/chips.md §3.6.36.

local common = require("@common/follow-effect/follow_effect")

local follow_effect = {}

-- A look, as a record (a spawner's).
follow_effect.look = common.look

follow_effect.kind = common.make_kind {
    id = "bn6:follow-effect",
    slide_sound = asset.sound("bn6:follow-effect"),
    copies_visibility = true,
}

-- `sub_80E3FB4`: a follow effect of `look` for `owner` at `pos`, with the
-- game's flip value `flip` and `palette_add`.
function follow_effect.spawn(owner: Object, pos: Vec3, look: FollowEffectLook, flip: number, palette_add: number?): Object?
    return common.spawn(follow_effect.kind, owner, pos, look, flip, palette_add)
end

-- `sub_80E3FC4`: end it (its next update frees it).
follow_effect.stop = common.stop

return follow_effect
