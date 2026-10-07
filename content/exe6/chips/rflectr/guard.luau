--!strict
-- The Reflectors' guard (the original's action 0x2B, `sub_80ED13E`; see
-- chips.md §3.3): the navi holds up a shield (shield.luau) and guards for
-- its ticks. The first hit the guard blocks from the front sends its
-- user's counterattack (`counter`): the Reflectors' wave back along the row
-- (init.luau) with the chip's damage, or the NaviCust Reflect's buster
-- shot (navis/megaman/weapons/shield); a hit that breaks guards takes the
-- shield down (the guard with it). The NaviCust Shield's guard heeds
-- nothing it blocks.

local shield = require("./shield")

local guard = {}

export type State = {
    -- Ticks guarded.
    timer: number,
}

-- One state for every guard.
guard.STATE = { timer = "u16" }

-- The shield sits at the navi's sprite attach point 6.
local SHIELD_POINT = 6
-- A hit that breaks guards (the collision's hit flag 2).
local BREAKS_GUARD = 0x2

-- The Beast forms whose head overlay mirrors the navi's animation: those
-- that wear a beast's head (the Falzar beast's, its Crosses' and Beast
-- Over's).
local function beast_head_follows(form: FormDef): boolean
    local identity = form.identity
    local parts = identity and identity.parts
    return parts ~= nil and parts.beast_head ~= nil
end

-- `sub_80101D4`: the Beast head (the navi's second related object) takes
-- animation `anim` too. (Without one, the game's stores go through a null
-- pointer and change nothing.)
local function set_head_animation(me: Object, anim: number)
    if beast_head_follows(battle.navi(me.alliance).form) then
        local head = me.related2
        if head then
            head.anim = anim
            head.anim_loaded = 0xFF
        end
    end
end

-- The shield goes, and the guard with it.
local function drop_shield(me: Object)
    me.related1 = nil
    me.overlay = nil
    me:set_status("guard", false)
end

-- A guard (`ReflectorGuard`, types.d.luau).
function guard.action(spec: ReflectorGuard): Action
    -- The counterattack, one panel ahead (`loc_80ED1DC`).
    local function counter(me: Object)
        local send = spec.counter
        if send then
            local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
            send(me, me.panel_x + me.facing, me.panel_y, damage)
        end
    end

    -- `sub_80ED154`: raise the shield; then, each tick, counter the first
    -- blocked hit, until the time is up.
    local function guarding(me: Object, s: State)
        if me.step_init == 0 then
            local x, z = me:attach_point(SHIELD_POINT)
            shield.spawn(me, Vec3.px(x, 0, z), spec.look)
            me:set_status("guard", true)
            me:set_status("using_action", true)
            me:set_animation(0)
            set_head_animation(me, 0)
            me.marker = 0
            me.step_init = 4
            s.timer = 0
            return
        end
        local blocked = bit32.btest(me.collision.guard_dirs, bit32.lshift(1, me.flip))
        if not spec.heedless and blocked then
            if bit32.btest(me.collision.hit_flags, BREAKS_GUARD) then
                drop_shield(me)
            elseif me.marker == 0 then
                me.marker = 1
                -- (A heedless guard's case here would add 0x580 to the
                -- side's custom gauge, `sub_802E032`; a heedless guard
                -- never gets here.)
                counter(me)
            end
        end
        local t = s.timer + 1
        s.timer = t
        if t > spec.ticks then
            drop_shield(me)
            me:exit_attack()
        end
    end

    -- `sub_80ED13E`: one phase.
    local function update(me: Object, s: State)
        local step = me.step
        if step ~= 0 then
            error(string.format("Reflector phase %#x reads past its phase table", step))
        end
        guarding(me, s)
    end

    return new.action { id = spec.id, state = guard.STATE, args = spec, update = update }
end

return guard
