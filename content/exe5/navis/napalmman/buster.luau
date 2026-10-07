--!strict
-- NapalmMan's buster (`napalmman/buster`): the original's weapon routine 0x44 (0x0800FBC0) sets the damage his level
-- gives (the damage rows' row 6, lib/navi_level), his own counter byte, no element and no bonus, and starts his own
-- action 7 (0x080F0F6C), a machine gun: he aims (animation 14, restarted each shot) and on the second tick a
-- bullet (objects/bullet, its row 0x13: the panel it finds and the one behind it, no flinch) flies from the panel
-- ahead at a height one draw picks, with a sound and the muzzle flash in his first related slot; after five ticks,
-- three more of recovery, which a held direction cuts short into a step (with no test of the panel first: the
-- step's own start finds none and ends). When the recovery runs out with B held, a player's NapalmMan shoots again
-- at once: the same attack, from its first step.
--
-- The routine also keeps the side's buster-shot program (NaviStats +0x4D) in the attack's first parameter, which
-- his action hands to its shot's routine in a register that routine overwrites: nothing reads it.

local attachment = require("../../objects/attachment")
local bullet = require("../../objects/bullet")
local bullets = require("@exelib/bullet")
local lib = require("../../lib/buster")
local navi_level = require("../../lib/navi_level")
local regions = require("@exelib/regions")

local buster = {}

export type State = {
    -- AIAttackVars +0x10: ticks into the shot, then the recovery's ticks left.
    timer: number,
}

local HIT_PARAM = 0x82
-- The damage rows' row 6, by his level: a bullet's damage.
local DAMAGE: { number } = { 5, 5, 5, 5, 5, 7, 7 }
local ANIM = 14
-- The tick the bullet flies on, the shot's last tick, and the recovery (AIAttackVars +0x12).
local SHOT_TICK, SHOT_TICKS, RECOVERY = 1, 4, 3
-- The bullet's heights, by a draw's low two bits (pixels: 0x080F1094).
local HEIGHTS = { [0] = 8, 16, 24, 32 }
local SOUND = asset.sound("col-army-2")
-- The bullet (row 0x13): the panel found and the one behind it, with no flinch, its hit shown once at the bullet
-- (effect 0x58: EXE5's own vulcan hit, its animation 1).
local SHOT = bullet.variant {
    region = regions.long,
    hit_mod = 0,
    effect = new.effect { sprite = asset.sprite("vulcan-hit"), anim = 1, palette = 0 },
    single = true,
}

-- The steps, on the attack's step.
local SHOOT, RECOVER = 0x0, 0x4

-- 0x080F1062: a bullet from the panel ahead, at a height a draw picks, of the attack's element, damage and bonus.
local function fire(me: Object)
    local z = HEIGHTS[bit32.band(battle.rng_positive(), 3)] * 0x1_0000
    local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
    bullets.fire(bullet.kind, me, me.panel_x + me.facing, me.panel_y, z, SHOT, me.attack_element, damage)
end

-- 0x080F0F9C: he aims; on the second tick the bullet, the recovery's length and the muzzle flash; after five ticks
-- the recovery.
local function shoot(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(ANIM)
        me:set_status("using_action", true)
        me.step_init = 4
        s.timer = 0
    end
    if s.timer == SHOT_TICK then
        battle.play_sound(SOUND)
        fire(me)
        me.attack_count = RECOVERY
        attachment.attach(me, lib.flash, "related")
    end
    local t = s.timer + 1
    s.timer = t
    if t > SHOT_TICKS then
        me.step, me.step_init = RECOVER, 0
    end
end

-- 0x080F0FF8: the recovery. A held direction cuts it short into a step. When it runs out the flash is let go, and a
-- player's NapalmMan with B held shoots again; else back to idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = me.attack_count
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = bit32.band(t, 0xFFFF)
    if t < 0 then
        me.related1 = nil
        if me.actor_type == "player" and me:held("b") then
            me.marker = 4
            me.step, me.step_init = SHOOT, 0
        else
            me:exit_attack()
        end
        return
    end
    if not me:can_move() then
        return
    end
    local dir = me:held_direction()
    if dir ~= 0 then
        me.related1 = nil
        me:exit_attack()
        me:start_move(dir)
    end
end

buster.action = new.action {
    id = "napalmman/buster/action",
    state = { timer = "u16" },
    -- 0x080F0F6C: its step; then a trap chip's catch takes over (the reactive defenses' check).
    update = function(me: Object, s: State)
        local step = me.step
        if step == SHOOT then
            shoot(me, s)
        elseif step == RECOVER then
            recover(me, s)
        else
            error(string.format("NapalmMan's buster's step %#x reads past its table (0x080F0F90)", step))
        end
        if me:request("anti_damage_triggered") or me:request("anti_sword_triggered") then
            me:check_reactive_abort()
        end
    end,
}

-- 0x0800FBC0
local function setup(navi: Object): Action
    navi.attack_damage = navi_level.damage(navi, DAMAGE)
    navi.hit_param = HIT_PARAM
    navi.attack_element = 0
    navi.charged = 0
    navi.extra = 0
    navi.attack_lockout = 0
    navi.attack_variant = 0
    navi.marker = 0
    return buster.action
end

buster.weapon = new.weapon {
    id = "napalmman/buster",
    -- The charge table's row 0x44 (0x0801CD14), which a buster doesn't read.
    charge_ticks = { 80, 75, 70, 65, 60, 60 },
    setup = setup,
}

return buster
