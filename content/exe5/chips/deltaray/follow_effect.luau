--!strict
-- EXE5's follow effect (effect object #0x31, 0x080E3E3C): the shared follow effect (@exelib/follow_effect), EXE5's
-- kind, which isn't shown as its owner is (0x080E3EB4 runs the look's routine alone). Its look and routine are its
-- first parameter's row (0x080E3E28, 0x080E3ED0); its flip and the palette added its second and third. Of EXE5's
-- looks only DeltaRay's burst (look 4, "flash", sprite 10-34) is here: looks 0 to 3 run EXE5's own routines, which
-- nothing ported spawns.

local common = require("@exelib/follow_effect")

local follow_effect = {}

follow_effect.kind = common.make_kind { id = "follow-effect", slide_sound = nil, copies_visibility = false }

-- Its looks by name.
follow_effect.looks = {
    -- Look 4: DeltaRay's burst.
    burst = common.look { sprite = asset.sprite("follow-effect"), anim = 0, palette = 0, behavior = "flash" },
}

-- 0x080E405E: a follow effect of `look` for `owner` at `pos`, with the game's flip value `flip` and `palette_add`.
function follow_effect.spawn(
    owner: Object,
    pos: Vec3,
    look: FollowEffectLook,
    flip: number,
    palette_add: number?
): Object?
    return common.spawn(follow_effect.kind, owner, pos, look, flip, palette_add)
end

return follow_effect
