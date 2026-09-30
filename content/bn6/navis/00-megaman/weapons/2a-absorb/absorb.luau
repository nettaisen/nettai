--!strict
-- DustCross's B+Back special: weapon routine 0x2A (`sub_8011F84`) and
-- action 0x58 (`sub_80EFCB4`), pulling in the field's obstacles. A vortex
-- (effect 0x63) appears at the navi; on the 10th tick every obstacle on the
-- field is pulled in (the obstacle framework's `sub_80EFD74`: each flies to
-- the navi as an absorbed obstacle, objects/absorbed-obstacle, and joins
-- its absorbed list while the navi is still in this action); 11 ticks
-- later the navi idles. See docs/engine/field-objects.md §4.4.

export type State = {
    -- Ticks left in the phase (AIAttackVars+0x10).
    timer: number,
    -- The vortex, kept alive while the action runs. (The game keeps it in
    -- the attack's word at +0x30, the marker.)
    vortex: Object?,
}

local absorb = {
    state = { timer = "u16", vortex = "object" },
}

local ACTION = 0x58

-- The phases, on the attack's step.
local PULL, RECOVER = 0x0, 0x4

-- The B+Back special's cooldown after.
local COOLDOWN = 0x28

-- The vortex effect, and what keeps it alive each tick.
local VORTEX, VORTEX_TICKS = 0x63, 2

-- DustCross's Beast form (the navi stats' form).
local DUST_BEAST = 0x16

local SOUND = 0xAD

-- `sub_8011F84`: only the cooldown.
function absorb.setup(navi: Object): number
    navi.attack_lockout = COOLDOWN
    return ACTION
end

-- `sub_80EFCD8`: the pose and the vortex, then the pull on the 10th tick.
local function pull(me: Object, s: State)
    if me.step_init == 0 then
        local beast = battle.navi(me.alliance).form == DUST_BEAST
        me:set_animation(if beast then 0x19 else 0x17)
        me:set_status("using_action", true)
        me:set_status("moving", true)
        -- Behind the navi's feet, or its Beast form's claws.
        local back, down = 7, 4
        if beast then
            back, down = 3, -10
        end
        local p = me.pos
        local at = Vec3.new(p.x - int.shl(me.facing * back, 16), p.y + int.shl(down, 16), p.z)
        local vortex = battle.effect(at, VORTEX, me.alliance)
        s.vortex = vortex
        if vortex then
            vortex.run_while_paused = false
            vortex.run_while_dimmed = false
        end
        battle.play_sound(SOUND)
        s.timer = 9
        me.step_init = 4
    end
    local left = s.timer - 1
    s.timer = left
    if left >= 0 then
        return
    end
    obstacle.absorb_all(me)
    s.timer = 0xB
    me.step = RECOVER
    me.step_init = 0
end

-- `sub_80EFD5E`: 11 ticks, then idle.
local function recover(me: Object, s: State)
    local left = s.timer - 1
    s.timer = left
    if left > 0 then
        return
    end
    me:set_status("moving", false)
    me:exit_attack()
end

-- `sub_80EFCB4`: the phase, then the vortex lives on while this runs.
function absorb.update(me: Object, s: State)
    local step = me.step
    if step == PULL then
        pull(me, s)
    elseif step == RECOVER then
        recover(me, s)
    else
        error(string.format("absorbing phase %#x reads past sub_80EFCB4's phase table", step))
    end
    local vortex = s.vortex
    if vortex then
        vortex.timer = VORTEX_TICKS
    end
end

return absorb
