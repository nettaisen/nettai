--!strict
-- The Falzar Beast forms' buster (`megaman/falzar-beast-buster`): the
-- original's weapon routine 3 (`sub_8011AF2`), also what DustCross Beast's
-- buster fires without an absorbed obstacle, and action 0x1E
-- (`sub_80EC5BC`). The navi raises the buster and fires the flying shot
-- (objects/flying-shot) from the panel in front. The shots of one attack
-- cycle single, volley of three, volley of three; in Beast Over (or with a
-- Cross special in the per-player gauge mode) the cycle starts at a volley
-- and eight more shots follow. Holding B keeps it firing. See
-- docs/engine/chips.md §4.6.

local buster = require("../../../../lib/buster")
local plain_buster = require("../buster/weapon")
local flying_shot = require("../../../../objects/flying-shot/flying_shot")
local weapon = require("../../../../lib/weapon")

export type State = {
    -- Ticks into the shot, then ticks of recovery (AIAttackVars+0x10).
    timer: number,
    -- The recovery after a shot (AIAttackVars+0x12).
    recovery: number,
    -- The shots still to follow (the attack's parameter byte +0x0D), and
    -- where the volley cycle is (+0x0E): 0 a single shot, else a volley.
    repeats: number,
    cycle: number,
}

local falzar_beast_buster = {}

-- What a Beast buster's action keeps of its setup: the two Beast busters'
-- states both have them.
export type Volleys = { repeats: number, cycle: number }

-- The phases, on the attack's step.
local SHOOT, RECOVER = 0x0, 0x4

-- Falzar Beast Over.
local BEAST_OVER = 0x18

local SOUND = asset.sound("aqua-needle-2")

-- The first shot's height (`dword_80EC70C`'s first byte), pixels.
local HEIGHT = 0x16

-- The volley's other shots: 30 pixels back, 24 to either side, the second
-- 12 lower.
local VOLLEY_BACK, VOLLEY_SIDE, VOLLEY_DROP = 0x1E_0000, 0x18_0000, 0xC_0000

-- The shots' palette by Beast form (`byte_80EC784`, forms 0xB..0x18).
local PALETTES = {
    [0xB] = 0, [0xC] = 0, [0xD] = 0, [0xE] = 0, [0xF] = 0, [0x10] = 0, [0x11] = 0,
    [0x12] = 1, [0x13] = 2, [0x14] = 3, [0x15] = 4, [0x16] = 5, [0x17] = 0, [0x18] = 0,
}

-- The repeats a Beast Over's buster (or a Cross special's) fires.
local REPEATS = 8

-- `sub_8011AF2` and `sub_8011B4A`: a Beast form's buster. Damage is the
-- buster's, at most 5. `over` is the form whose volleys repeat; `volleys`
-- the state of the action about to start. (The game also puts the
-- NaviCust's buster-shot program in the attack's first parameter, which
-- neither Beast buster reads.)
function falzar_beast_buster.beast_setup(navi: Object, over: number, volleys: Volleys)
    navi.attack_damage = math.min(navi:buster_damage(), 5)
    navi.hit_param = 0
    navi.attack_element = 0
    navi.charged = 0
    navi.extra = 0
    navi.attack_lockout = 0
    navi.marker = 0
    local cross_special = battle.per_player_gauges() and battle.side_special(navi.alliance) == "cross"
    if cross_special or battle.navi(navi.alliance).form == over then
        volleys.cycle = 1
        volleys.repeats = REPEATS
    else
        volleys.cycle = 0
        volleys.repeats = 0
    end
    -- AIData+0x68: the arm slot is let go.
    navi.overlay = nil
end

-- `sub_8011AF2`.
function falzar_beast_buster.setup(navi: Object): Action
    falzar_beast_buster.beast_setup(navi, BEAST_OVER, navi:action_state(falzar_beast_buster.action) :: State)
    return falzar_beast_buster.action
end

-- `sub_80EC710`: the shot from the panel in front, and with the volley
-- cycle on, the two behind it. (`sub_80C6248`: its spawner's word is the
-- shot's height here, which only a thrown obstacle reads.)
local function shoot_volley(me: Object, s: State, falls: boolean, z: number, damage: number)
    local form = battle.navi(me.alliance).form
    local palette = PALETTES[form]
    if palette == nil then
        error(string.format("form %#x reads outside sub_80EC710's palette table", form))
    end
    local x, y = field.center(me.panel_x + me.facing, me.panel_y)
    local variants = flying_shot.variants
    flying_shot.launch(me, Vec3.new(x, y, z), { variant = variants.beast_shot, falls = falls, palette = palette, damage = damage })
    if s.cycle == 0 then
        return
    end
    local back = x - me.facing * VOLLEY_BACK
    local shot = { variant = variants.beast_volley_shot, falls = falls, palette = palette, damage = damage }
    flying_shot.launch(me, Vec3.new(back, y - VOLLEY_SIDE, z), shot)
    flying_shot.launch(me, Vec3.new(back, y + VOLLEY_SIDE, z - VOLLEY_DROP), shot)
end

-- `sub_80EC792`: the arm goes and the navi idles.
local function finish(me: Object)
    me.related1 = nil
    weapon.end_arm(me)
    me:exit_attack()
end

-- `sub_80EC5EC`: raise the buster (the Beast head's animation restarts);
-- the shot on the second tick; after 5 ticks, the recovery.
local function shoot(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(0x0E)
        -- (The game writes the head's byte without checking it's there.)
        local head = me.related2
        if head then
            head.anim_loaded = 0
        end
        me.related1 = nil
        weapon.end_arm(me)
        me.overlay = nil
        buster.raise_arm(me)
        me:set_status("using_action", true)
        me.step_init = 4
        s.timer = 0
    end
    if s.timer == 1 then
        battle.play_sound(SOUND)
        -- Airborne, the shots fall from the navi's height.
        local z, falls = HEIGHT * 0x10000, false
        if me.pos.z ~= 0 then
            falls = true
            z += me.pos.z
        end
        -- The damage word plus the bonus.
        local damage = int.u32(me.attack_damage + bit32.lshift(me.hit_param, 16) + me.extra)
        shoot_volley(me, s, falls, z, damage)
        s.recovery = 2
        local cycle = s.cycle + 1
        s.cycle = if cycle < 3 then cycle else 0
    end
    local t = s.timer + 1
    s.timer = t
    if t > 4 then
        me.step = RECOVER
        me.step_init = 0
    end
end

-- `sub_80EC67E`: the recovery, which a move may cut short; then another
-- shot while repeats are left, or (in a Beast form other than Beast Over,
-- for a player holding B) while B is held.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = s.recovery
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = t
    if t >= 0 then
        local dir = weapon.move_input(me)
        if dir then
            finish(me)
            me:start_move(dir)
        end
        return
    end
    local repeats = s.repeats
    if repeats ~= 0 then
        repeats -= 1
        s.repeats = repeats
        if repeats ~= 0 then
            me.marker = 4
            me.step = SHOOT
            me.step_init = 0
            return
        end
    end
    if battle.navi(me.alliance).form == BEAST_OVER or me:request("alt_chip") or me.actor_type ~= "player" then
        return finish(me)
    end
    if not me:held("b") then
        return finish(me)
    end
    me.marker = 4
    me.step = SHOOT
    me.step_init = 0
end

-- `sub_80EC5BC`: the phase, then the reactive-defense abort.
local function update(me: Object, s: State)
    local step = me.step
    if step == SHOOT then
        shoot(me, s)
    elseif step == RECOVER then
        recover(me, s)
    else
        error(string.format("Falzar Beast buster phase %#x reads past its phase table", step))
    end
    me:check_reactive_abort()
end

falzar_beast_buster.action = define.action {
    id = "megaman/falzar-beast-buster/action",
    state = { timer = "u16", recovery = "u16", repeats = "u8", cycle = "u8" },
    update = update,
}

falzar_beast_buster.weapon = define.weapon {
    id = "megaman/falzar-beast-buster",
    name = "Falzar Beast buster",
    -- The charge table's row 3, by Charge 0 to 7: past 4 it reads on
    -- into the next routine's row, as the original's table does.
    charge_ticks = { 40, 60, 90, 120, 150, 80, 75, 70 },
    setup = falzar_beast_buster.setup,
    -- As a buster it fires while B is held, and gives way to the plain
    -- buster while the charged shot is a chip's (a sticky one).
    held = true,
    plain = plain_buster.weapon,
}

return falzar_beast_buster
