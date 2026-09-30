--!strict
-- The Gregar Beast forms' buster: weapon routine 4 (`sub_8011B4A`) and
-- action 0x1D (`sub_80EC45E`). The navi raises the buster and fires the
-- buster's projectile (attack object #0, group D's kind) at a random
-- height from the panel in front, with the muzzle flash; a move may cut a
-- shot short from its fourth tick. In Beast Over (or with a Cross special
-- in the battle flag 0x40 mode) eight more shots follow; holding B keeps
-- it firing. See docs/engine/chips.md §4.6.

local attachment = require("../../../../objects/attachment/attachment")
local buster = require("../../../../lib/buster")
local falzar_beast_buster = require("../03-falzar-beast-buster/falzar_beast_buster")
local weapon = require("../../../../lib/weapon")

export type State = {
    -- Ticks into the shot (AIAttackVars+0x10).
    timer: number,
    -- The shot's length (AIAttackVars+0x12).
    length: number,
}

local gregar_beast_buster = {
    state = { timer = "u16", length = "u16" },
}

local ACTION = 0x1D

-- The phases, on the attack's step.
local SHOOT, NEXT = 0x0, 0x4

-- Gregar Beast Over.
local BEAST_OVER = 0x17

-- The projectile (attack object #0) and its kind (its Param1,
-- `dword_80EC5B4`: 6 either way).
local PROJECTILE = 0x00
local KINDS = { [0] = 6, [1] = 6 }

-- The shot's height by the draw (`dword_80EC5B0`), pixels.
local HEIGHTS = { [0] = 0x08, [1] = 0x10, [2] = 0x18, [3] = 0x20 }

-- The muzzle flash (data.objects.attachments).
local MUZZLE_FLASH = 5

-- `sub_8011B4A`.
function gregar_beast_buster.setup(navi: Object): number
    falzar_beast_buster.beast_setup(navi, BEAST_OVER)
    return ACTION
end

-- `sub_800FAAC` (`sub_80C4FFE`, `sub_801155A`): the buster's projectile
-- from the panel in front. Its element is what the caller left in r2 (here
-- the bonus's low byte), its position the registers until its init places
-- it. (The recovery `sub_800FAAC` also counts is unused here.)
local function projectile(me: Object, kind: number, damage: number, r2: number, z: number)
    local x, y = me.panel_x + me.facing, me.panel_y
    local o = battle.spawn("attack", PROJECTILE, Vec3.new(y, r2, z), { kind, 0, 0, 0 })
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = r2
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.alliance, o.flip = me.alliance, me.flip
        o.related1 = me
    end
end

-- `sub_80EC490`: raise the buster; the shot on the second tick; after
-- `length` ticks the next phase. From the fourth tick a move cuts it
-- short.
local function shoot(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(0x0E)
        buster.raise_arm(me)
        me:set_status("using_action", true)
        me.step_init = 4
        s.length = 6 - 2
        s.timer = 0
    end
    if s.timer == 1 then
        battle.play_sound(0xB9)
        local r = bit32.band(battle.rng_positive(), 3)
        local z = HEIGHTS[r] * 0x10000
        local kind = KINDS[bit32.band(r, 1)]
        local damage = int.u32(me.attack_damage + bit32.lshift(me.hit_param, 16) + me.extra)
        projectile(me, kind, damage, me.extra, z)
        attachment.spawn(me, { kind = MUZZLE_FLASH }, "related")
    end
    local t = s.timer + 1
    s.timer = t
    if t > s.length then
        me.step = NEXT
        me.step_init = 0
        return
    end
    if s.timer < 3 then
        return
    end
    local dir = weapon.move_input(me)
    if dir then
        me.related1 = nil
        weapon.end_arm(me)
        me:exit_attack()
        me:start_move(dir)
    end
end

-- `loc_80EC594`: the next shot starts this tick.
local function again(me: Object, s: State)
    me.marker = 4
    me.step = SHOOT
    me.step_init = 0
    shoot(me, s)
end

-- `sub_80EC556`: the arm goes; another shot while repeats are left, or
-- (outside Beast Over, for a player holding B) while B is held.
local function next_shot(me: Object, s: State)
    me.related1 = nil
    weapon.end_arm(me)
    local repeats = me:attack_param(2)
    if repeats ~= 0 then
        repeats -= 1
        me:set_attack_param(2, repeats)
        if repeats ~= 0 then
            return again(me, s)
        end
    end
    if battle.navi(me.alliance).form == BEAST_OVER or me:request("alt_chip") or me.actor_type ~= "player" then
        return me:exit_attack()
    end
    if not me:held("b") then
        return me:exit_attack()
    end
    again(me, s)
end

-- `sub_80EC45E`: the phase, then the reactive-defense abort.
function gregar_beast_buster.update(me: Object, s: State)
    local step = me.step
    if step == SHOOT then
        shoot(me, s)
    elseif step == NEXT then
        next_shot(me, s)
    else
        error(string.format("Gregar Beast buster phase %#x reads past its phase table", step))
    end
    me:check_reactive_abort()
end

return gregar_beast_buster
