--!strict
-- The Spreaders' action, as BN5 and BN6 share it (BN6's action 0x25,
-- `sub_80ECBB0`, of a game's `SpreaderLook`: its sound, attachment kind and
-- bullets' kind;
-- docs/engine/shot-chips.md §5): the navi raises its gun and on the second
-- tick fires a bullet (@exelib/bullet) from the panel ahead, which bursts
-- where it finds something; 21 ticks of recovery. SpoutCross's charged
-- shot fires its own bullet from its water cannon the same way.

local attachment = require("../attachment/attachment")
local bullet = require("../bullet/bullet")

local spreader = {}

export type State = {
    -- Ticks since the gun came up, then the recovery's countdown.
    timer: number,
}

-- One state for every spreader.
spreader.STATE = { timer = "u16" }

-- The phases, on the attack's step.
local FIRE, RECOVER = 0x0, 0x4

local ANIM = 0xB
-- The bullet leaves once the count passes this; the recovery's ticks.
local FIRE_AFTER, RECOVER_TICKS = 1, 0x14

-- `sub_80ECC86`: the gun's animation by the holder's AI index (the link
-- navis HeatMan, GroundMan and DustMan, 1, 9 and 10: 2; the others 0).
local function gun_anim(me: Object): number
    local ai = me.ai_index
    if ai == 1 or ai == 9 or ai == 10 then
        return 2
    end
    if ai > 27 then
        error(string.format("AI index %d reads past off_80ECC94", ai))
    end
    return 0
end

-- A chip's spreader (a `SpreaderSpec`), of a game's `look`.
function spreader.action(spec: SpreaderSpec, look: SpreaderLook): Action
    -- `sub_80ECBCC`: raise the gun; on the second tick, the bullet.
    local function fire(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(ANIM)
            me:open_counter_window()
            attachment.attach(look.attachment, me, spec.gun, "overlay", { anim = gun_anim(me) })
            me:set_status("using_action", true)
            me.step_init = 4
            s.timer = 0
        end
        local t = s.timer + 1
        s.timer = t
        if t <= (spec.fire_after or FIRE_AFTER) then
            return
        end
        local gun = me.overlay
        if gun then
            gun.anim += 1
        end
        battle.play_sound(spec.sound or look.sound)
        local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
        bullet.fire(spec.bullet_kind or look.bullet_kind, me, me.panel_x + me.facing, me.panel_y, spec.z, spec.bullet, me.attack_element, damage)
        me.step, me.step_init = RECOVER, 0
    end

    -- `sub_80ECC60`: 21 ticks, then the gun goes and back to idle.
    local function recover(me: Object, s: State)
        if me.step_init == 0 then
            s.timer = RECOVER_TICKS
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = t
        if t < 0 then
            me.related1 = nil
            me.overlay = nil
            me:exit_attack()
        end
    end

    return define.action {
        id = spec.id,
        state = spreader.STATE,
        args = spec,
        -- `sub_80ECBB0`.
        update = function(me: Object, s: State)
            local step = me.step
            if step == FIRE then
                fire(me, s)
            elseif step == RECOVER then
                recover(me, s)
            else
                error(string.format("spreader phase %#x reads past off_80ECBC4", step))
            end
        end,
    }
end

return spreader
