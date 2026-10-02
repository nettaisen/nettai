--!strict
-- The Gregar Beast forms' buster (`megaman/gregar-beast-buster`): the
-- original's weapon routine 4 (`sub_8011B4A`) and action 0x1D
-- (`sub_80EC45E`). The navi raises the buster and fires the projectile
-- (objects/projectile) at a random height from the panel in front, with the
-- muzzle flash; a move may cut a shot short from its fourth tick. In Beast
-- Over (or with a Cross special in the per-player gauge mode) eight more
-- shots follow; holding B keeps it firing. See docs/engine/chips.md §4.6.

local attachment = require("../../../../objects/attachment/attachment")
local buster = require("../../../../lib/buster")
local falzar_beast_buster = require("../falzar-beast-buster/weapon")
local plain_buster = require("../buster/weapon")
local projectile = require("../../../../lib/projectile")
local variants = require("../../../../objects/projectile/variants")
local weapon = require("../../../../lib/weapon")

export type State = {
    -- Ticks into the shot (AIAttackVars+0x10). (The shot's length is the
    -- navi's `attack_count`.)
    timer: number,
    -- The shots still to follow (the attack's parameter byte +0x0D), and
    -- the volley cycle its setup shares with the Falzar buster's (+0x0E),
    -- which this action never reads.
    repeats: number,
    cycle: number,
}

local gregar_beast_buster = {}

-- The phases, on the attack's step.
local SHOOT, NEXT = 0x0, 0x4

-- Gregar Beast Over.
local BEAST_OVER = "gregar-beast-over"

local SOUND = asset.sound("col-army-2")

-- The projectile, whichever way the draw's low bit falls (`dword_80EC5B4`:
-- the charged shot's either way).
local SHOTS = { [0] = variants.charged, [1] = variants.charged }

-- The shot's height by the draw (`dword_80EC5B0`), pixels.
local HEIGHTS = { [0] = 0x08, [1] = 0x10, [2] = 0x18, [3] = 0x20 }

-- `sub_8011B4A`.
function gregar_beast_buster.setup(navi: Object): Action
    falzar_beast_buster.beast_setup(navi, BEAST_OVER, navi:action_state(gregar_beast_buster.action) :: State)
    return gregar_beast_buster.action
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
        me.attack_count = 6 - 2
        s.timer = 0
    end
    if s.timer == 1 then
        battle.play_sound(SOUND)
        local r = bit32.band(battle.rng_positive(), 3)
        local z = HEIGHTS[r] * 0x10000
        local damage = int.u32(me.attack_damage + bit32.lshift(me.hit_param, 16) + me.extra)
        -- `sub_800FAAC`: the projectile from the panel in front. (The
        -- game's spawner also stores what the caller left in r2, the
        -- bonus's low byte, as its element, which the projectile's init
        -- replaces with its variant's; the recovery it counts is unused
        -- here.)
        projectile.fire(me, { variant = SHOTS[bit32.band(r, 1)], damage = damage, z = z })
        attachment.attach(me, buster.flash, "related")
    end
    local t = s.timer + 1
    s.timer = t
    if t > me.attack_count then
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
    local repeats = s.repeats
    if repeats ~= 0 then
        repeats -= 1
        s.repeats = repeats
        if repeats ~= 0 then
            return again(me, s)
        end
    end
    if battle.navi(me.alliance).form.id == BEAST_OVER or me:request("alt_chip") or me.actor_type ~= "player" then
        return me:exit_attack()
    end
    if not me:held("b") then
        return me:exit_attack()
    end
    again(me, s)
end

-- `sub_80EC45E`: the phase, then the reactive-defense abort.
local function update(me: Object, s: State)
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

gregar_beast_buster.action = define.action {
    id = "megaman/gregar-beast-buster/action",
    state = { timer = "u16", repeats = "u8", cycle = "u8" },
    update = update,
}

gregar_beast_buster.weapon = define.weapon {
    id = "megaman/gregar-beast-buster",
    name = "Gregar Beast buster",
    -- The charge table's row 4, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 80, 75, 70, 65, 60, 20, 20, 20 },
    setup = gregar_beast_buster.setup,
    -- As a buster it fires while B is held, and gives way to the plain
    -- buster while the charged shot is a chip's (a sticky one).
    held = true,
    plain = plain_buster.weapon,
}

return gregar_beast_buster
