--!strict
-- EXE4's follow effect (effect object #0x31, 0x080E58B4): the shared follow effect (@exelib/follow_effect), EXE4's
-- kind, which isn't shown as its owner is (0x080E592C runs the look's routine alone, as EXE5's). Its look and routine
-- are its first parameter's row (0x080E589C, 0x080E5948); its flip and the palette added its second and third. Of
-- EXE4's looks DeltaRay's burst (look 4, "flash", sprite 10-34) and MetalMan's fist (look 5, "with_idle_owner", EXE4's
-- own routine 0x080E5AC8, sprite 10-39) are here: nothing ported spawns the others.

local common = require("@exelib/follow_effect")

local follow_effect = {}

follow_effect.kind = common.make_kind { id = "follow-effect", slide_sound = nil, copies_visibility = false }

-- Its looks by name.
follow_effect.looks = {
    -- Look 4: DeltaRay's burst.
    burst = common.look { sprite = asset.sprite("follow-effect"), anim = 0, palette = 0, behavior = "flash" },
    -- Look 5: MetalMan's fist.
    metal_fist = common.look {
        sprite = asset.sprite("metalman-fist"),
        anim = 0,
        palette = 0,
        behavior = "with_idle_owner",
    },
}

-- 0x080E5B34: a follow effect of `look` for `owner` at `pos`, with the game's flip value `flip` and `palette_add`.
function follow_effect.spawn(
    owner: Object,
    pos: Vec3,
    look: FollowEffectLook,
    flip: number,
    palette_add: number?
): Object?
    return common.spawn(follow_effect.kind, owner, pos, look, flip, palette_add)
end

-- 0x080E5B44: end it (its next update frees it).
follow_effect.stop = common.stop

return follow_effect
