--!strict
-- The cannons' action (the original's action 0x14, `sub_80EBC0E`): Cannon,
-- HiCannon, M-Cannon and the GigaCans. The navi raises its cannon (an
-- attachment in its first related slot), and on the 17th tick fires a
-- projectile from the panel ahead, 24 pixels up; at the 30th it lowers the
-- cannon (animation 7) and is back to idle 4 ticks later. A GigaCan leaves
-- two afterimages on the 10th tick, of its cannon and of the navi. Each
-- chip composes its own (`cannon.action { arm = ..., shot = ... }`): what
-- it holds and what it fires (docs/design/content-model-v2.md §5.3,
-- docs/engine/shot-chips.md §2).

local attachment = require("../objects/attachment/attachment")
local projectile = require("./projectile")

local cannon = {}

export type State = {
    -- Ticks since the cannon came up, then the lowering's countdown.
    timer: number,
}

-- One state for every cannon: a cannon continues the last one's attack
-- variables, as the original's one action does.
cannon.STATE = { timer = "u16" }

-- The phases, on the attack's step.
local FIRE, LOWER = 0x0, 0x4

local ANIM_RAISE, ANIM_LOWER = 8, 7
local SOUND = asset.sound("bn6:cannon")
-- The shot leaves on this tick of the count, and the phase ends on this
-- one; the afterimages come on this one.
local SHOT_TICK, FIRE_TICKS, AFTERIMAGE_TICK = 0xF, 0x1D, 8
local LOWER_TICKS = 3
-- The shot's height, 16.16.
local SHOT_Z = 0x18_0000

-- The afterimages: their color shader and lifetime, the navi's attach
-- point the cannon's sits at, the navi's animation the other holds.
local AFTERIMAGE_SHADER, AFTERIMAGE_TICKS = 0x8318, 0x1E
local CANNON_POINT, NAVI_ANIM = 7, 8

-- The cannons' looks: the arm the navi holds (the original's attachment
-- rows 0 to 2, by `byte_80EBD2C`), at its attach point 7.
local CANNON = asset.sprite("bn6:cannon")
cannon.arms = {
    cannon = attachment.look { sprite = CANNON, palette = 0, attach_point = 7 },
    hicannon = attachment.look { sprite = CANNON, palette = 1, attach_point = 7 },
    m_cannon = attachment.look { sprite = CANNON, palette = 2, attach_point = 7 },
}

-- `sub_80EBD68`: the arm's animation by who holds it: a link navi as a
-- player shows its own (HeatMan, GroundMan and DustMan, AI indices 1, 9
-- and 10: 1), anyone else the plain one.
local function arm_anim(me: Object): number
    if me.actor_type ~= "player" then
        return 0
    end
    local ai = me.ai_index
    if ai == 1 or ai == 9 or ai == 10 then
        return 1
    end
    if ai > 27 then
        error(string.format("AI index %d reads past off_80EBD80", ai))
    end
    return 0
end

-- The attack's damage word with the Atk+ bonus (`av.u32[8] + av.u16[6]`).
local function damage_word(me: Object): number
    return int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
end

local function set_phase(me: Object, step: number)
    me.step = step
    me.step_init = 0
end

-- A chip's cannon.
function cannon.action(spec: CannonSpec): Action
    -- GigaCan's afterimages (`sub_80E33FA`): of its cannon at the navi's
    -- attach point 7, and of the navi holding animation 8.
    local function afterimages(me: Object)
        local x, z = me:attach_point(CANNON_POINT)
        local p = me.pos
        -- `object_getFlip`: the side's facing and the object's flip.
        local flip = bit32.bxor(me.alliance, me.flip)
        battle.afterimage(me, Vec3.new(p.x + x * 0x10000, p.y, p.z + z * 0x10000), {
            sprite = spec.arm.sprite,
            anim = 0,
            flip = flip,
            lifetime = AFTERIMAGE_TICKS,
            color_shader = AFTERIMAGE_SHADER,
        })
        battle.afterimage(me, me.pos, {
            anim = NAVI_ANIM,
            flip = flip,
            lifetime = AFTERIMAGE_TICKS,
            color_shader = AFTERIMAGE_SHADER,
            shadow = "ground",
        })
    end

    -- `sub_80EBC28`: raise the cannon; fire on the count's 0xF; lower it at
    -- 0x1D.
    local function fire(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(ANIM_RAISE)
            me:open_counter_window()
            s.timer = 0
            attachment.attach(me, spec.arm, "related", { anim = arm_anim(me) })
            me:set_status("using_action", true)
            me.step_init = 4
            return
        end
        local t = s.timer
        if t == AFTERIMAGE_TICK and spec.afterimages then
            afterimages(me)
        end
        if t == SHOT_TICK then
            battle.play_sound(SOUND)
            local x, y = me.panel_x + me.facing, me.panel_y
            projectile.spawn(me, x, y, { variant = spec.shot, damage = damage_word(me), z = SHOT_Z })
        end
        t += 1
        s.timer = t
        if t == FIRE_TICKS then
            set_phase(me, LOWER)
        end
    end

    -- `sub_80EBD3C`: let go of the cannon, lower the arm, 4 ticks.
    local function lower(me: Object, s: State)
        if me.step_init == 0 then
            me.related1 = nil
            me.overlay = nil
            me:set_animation(ANIM_LOWER)
            s.timer = LOWER_TICKS
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = t
        if t < 0 then
            me:exit_attack()
        end
    end

    -- `sub_80EBC0E`.
    local function update(me: Object, s: State)
        local step = me.step
        if step == FIRE then
            fire(me, s)
        elseif step == LOWER then
            lower(me, s)
        else
            error(string.format("cannon phase %#x reads past off_80EBC20", step))
        end
    end

    return define.action { id = spec.id, state = cannon.STATE, args = spec, update = update }
end

return cannon
