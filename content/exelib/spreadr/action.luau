--!strict
-- The Spreaders' action, as EXE5 and EXE6 share it (EXE6's action 0x25,
-- `sub_80ECBB0`; EXE4's 0x1E, 0x080EC760), of a game's `SpreaderLook`: its
-- sound, how the navi holds the gun, its bullets' kind and the navi's
-- animation;
-- docs/engine/shot-chips.md §5): the navi raises its gun and on the second
-- tick fires a bullet (@exelib/bullet) from the panel ahead, which bursts
-- where it finds something; 21 ticks of recovery. SpoutCross's charged
-- shot fires its own bullet from its water cannon the same way.

local bullet = require("../bullet")
local slot = require("../slot")

local spreader = {}

export type State = {
    -- Ticks since the gun came up, then the recovery's countdown.
    timer: number,
}

-- One state for every spreader.
spreader.STATE = { timer = "u16" }

-- The phases, on the attack's step.
local FIRE, RECOVER = 0x0, 0x4

-- The bullet leaves once the count passes this; the recovery's ticks.
local FIRE_AFTER, RECOVER_TICKS = 1, 0x14

-- A chip's spreader (a `SpreaderSpec`), of a game's `look`.
function spreader.action(spec: SpreaderSpec, look: SpreaderLook): Action
    -- `sub_80ECBCC`: raise the gun; on the second tick, the bullet.
    local function fire(me: Object, s: State)
        if me.step_init == 0 then
            me:set_animation(look.anim)
            me:open_counter_window()
            look.hold(me, spec.gun, look.slot)
            me:set_status("using_action", true)
            me.step_init = 4
            s.timer = 0
        end
        local t = s.timer + 1
        s.timer = t
        if t <= (spec.fire_after or FIRE_AFTER) then
            return
        end
        local gun = slot.held(me, look.slot)
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

    return new.action {
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
