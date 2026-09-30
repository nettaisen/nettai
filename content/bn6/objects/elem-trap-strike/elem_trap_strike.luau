--!strict
-- ElemTrap's counterattack (effect object #0x2B, `sub_80E35A4`): a dimming
-- controller its trap (objects/elem-trap) starts when it springs, at the
-- head of the update list. The usual dimming phases (the dimming service),
-- with an effect that strikes every panel the enemy stands on in the
-- element that sprang the trap (the damage plus the bonus; elec and wood
-- hits carry a status too), sets off bursts over the whole field
-- (objects/panel-bursts), and waits 61 ticks.

local panel_bursts = require("../panel-bursts/panel_bursts")

export type State = {
    -- The chip and the Atk+ / cross bonus (object +0x30, +0x32); the bonus
    -- adds to the damage.
    chip: number,
    bonus: number,
}

local elem_trap_strike = {
    state = { chip = "u16", bonus = "u16" },
}

local WAIT = 0x3C

-- `byte_80E36E4`: the panels struck, by side: any with the enemy's navi,
-- other bodies or neutral objects (flags 0x01A00000 for side 0,
-- 0x02C00000 for side 1).
local STRUCK: { [number]: number } = { [0] = 0x01A0_0000, [1] = 0x02C0_0000 }

-- `byte_80E3714`: the strike's sound by element (0 = none).
local SOUNDS: { [number]: number } = { [0] = 0, [1] = 0x10F, [2] = 0x11D, [3] = 0x12E, [4] = 0xFF }

-- `byte_80E36EC`: the hit by element: region, hit spark, collision types
-- (target, self), hit modifier and status.
local HITS: { [number]: { hit_effect: number, hit_mod: number, status: number } } = {
    [0] = { hit_effect = 0x0A, hit_mod = 3, status = 0 },
    [1] = { hit_effect = 0x01, hit_mod = 3, status = 0 },
    [2] = { hit_effect = 0x02, hit_mod = 3, status = 0 },
    [3] = { hit_effect = 0x03, hit_mod = 1, status = 0x10 },
    [4] = { hit_effect = 0x04, hit_mod = 1, status = 0x20 },
}
local REGION, TARGET_TYPE, SELF_TYPE = 1, 5, 4

-- `byte_80E371E`: the bursts' look by element (objects/panel-bursts), over
-- the whole field (region 0x80), one every 2 ticks.
local BURST_LOOKS: { [number]: number } = { [0] = 3, [1] = 3, [2] = 0, [3] = 2, [4] = 4 }
local BURST_REGION, BURST_INTERVAL = 0x80, 2

-- `sub_80E360E`: the counterattack for `user`, of `element`, with the
-- trap's damage word and chip, at the head of the update list (it first
-- runs next tick). (Its position and parameters are the spawner's
-- registers, which nothing reads.)
function elem_trap_strike.spawn(user: Object, element: number, damage: number, chip: number, bonus: number): Object?
    local o = battle.spawn_kind_first("elem-trap-strike", Vec3.new(0, element, 0))
    if not o then
        return nil
    end
    o.element = element
    o.related1 = user
    o.alliance = user.alliance
    o.damage = damage
    o.stamina = bit32.rshift(damage, 16)
    local s = o.state :: State
    s.chip, s.bonus = chip, bonus
    return o
end

-- `sub_80E362C`: strike the panels, from (6, 3) back to (1, 1), then the
-- bursts. (Nothing to strike: nothing happens.)
local function strike(me: Object)
    local mask = STRUCK[me.alliance]
    local list = {}
    for x = 1, 6 do
        for y = 1, 3 do
            if bit32.btest(field.flags(x, y), mask) then
                table.insert(list, { x = x, y = y })
            end
        end
    end
    if #list == 0 then
        return
    end
    local e = me.element
    local sound = SOUNDS[e]
    local hit = HITS[e]
    local look = BURST_LOOKS[e]
    if sound == nil or hit == nil or look == nil then
        error(string.format("ElemTrap's counterattack: element %d reads past its tables", e))
    end
    if sound ~= 0 then
        battle.play_sound(sound)
    end
    local s = me.state :: State
    local word = int.u32(bit32.bor(me.damage, bit32.lshift(me.stamina, 16)) + s.bonus)
    for i = #list, 1, -1 do
        local p = list[i]
        local box = battle.hitbox(me, {
            panel_x = p.x,
            panel_y = p.y,
            element = e,
            z = 0,
            region = REGION,
            hit_effect = hit.hit_effect,
            target = TARGET_TYPE,
            self_type = SELF_TYPE,
            damage = bit32.band(word, 0xFFFF),
            stamina = bit32.rshift(word, 16),
            hit_mod = hit.hit_mod,
            status = hit.status,
        })
        if box then
            box.run_while_dimmed = true
        end
    end
    panel_bursts.spawn(me, BURST_REGION, BURST_INTERVAL, 0, { look })
end

-- `sub_80E35E8`: the strike, then 61 ticks.
local function effect(me: Object)
    if me.phase_init == 0 then
        me.timer = WAIT
        strike(me)
        me.phase_init = 4
    end
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        me:set_action(0xC)
    end
end

-- `sub_80E35A4`: the dimming's phases around the effect.
function elem_trap_strike.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        dimming.begin(me)
    elseif l == "update" then
        local a = me.action
        if a == 0 then
            dimming.dim_screen(me)
        elseif a == 4 then
            dimming.show_telop(me)
        elseif a == 8 then
            effect(me)
        elseif a == 0xC then
            dimming.undim_screen(me)
        else
            error(string.format("ElemTrap's counterattack action %#x reads past its table", a))
        end
    else
        dimming.finish(me)
    end
end

return elem_trap_strike
