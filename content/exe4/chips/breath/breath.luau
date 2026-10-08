--!strict
-- The breath chips' action (0x33, 0x080ED412) and breath (attack object 0x36, 0x080D3A7C): EXE4's own, HeatBrth's,
-- Blizzard's, ElecShok's and WoodPwdr's, each chip's own (`breath.action`, `breath.make`: its look, the original's row
-- of each of the object's tables by the chip's parameter, and its own routines where those tables name one).
--
-- The action: the navi (animation 13, a counter window) holds the chip's nozzle (attachment rows 0x18 to 0x1B by the
-- attack's element: breath-nozzle in palettes 0, 2, 4, 6; its animation three times the navi's number) for 37 ticks;
-- 6 ticks in, the nozzle's animation goes two on and the breath comes out on the panel ahead; then 6 ticks more, and
-- back to idle without it.
--
-- The breath, unseen itself, lasts 55 ticks with its sound (0x11F) as it starts. Where it hits a panel (a solid one:
-- 0x080D3B6C) its effect shows on the ground (turned: the effect's second parameter 1) and a hit of the chip's element
-- lands there for 40 ticks; on its own panel at once, then (40 ticks in, when that one was hit) on the column ahead,
-- each of its three panels. Its chip's step each tick, and its sound while its panel was hit; 15 ticks before its end
-- its own panel, and on its last tick the column's hit panels, show the panel effect and turn to the chip's panel
-- type (0x080D3DB6). (A breath of a parameter of 4 or more would also leave a trail behind it as it starts and 8
-- ticks in, 0x080D3C10: no chip's is. Its end clears its collision's region, 0x08012DC0: it has none.)

local attachment = require("../../objects/attachment")
local collision = require("../../rules/collision")
local regions = require("@exelib/regions")

local breath = {}

-- A chip's breath: its key; its hit's spark, element, hit modifier and status (0x080D3BE4, 0x080D3C0C, 0x080D3C00);
-- the effect where it hits (0x080D3BF8) and whether that effect lasts 40 ticks (HeatBrth's and ElecShok's); the panel
-- effect (none: WoodPwdr's) and type it leaves (0x080D3DEC, 0x080D3DF4); its own step each tick (0x080D3C5C's
-- routine: what it does to its panels as it goes, before it leaves them, which every breath does: 0x080D3DF8) and its
-- sound (0x080D3E64's).
type BreathLook = {
    id: string,
    spark: Spark,
    element: number,
    hit_mod: number,
    status: Status?,
    hit_effect: Effect,
    hit_effect_lasts: boolean,
    panel_effect: Effect?,
    panel: PanelType,
    step: ((me: Object, s: BreathState) -> ())?,
    sound: (me: Object, s: BreathState) -> (),
}

local TICKS, HIT_TICKS, COLUMN_AT = 55, 40, 40
local LEAVE_AT, LAST = 15, 1
-- Its sound as it starts (0x11F).
local SOUND = asset.sound("breath")
-- What a breath's effects show turned (the effect's second parameter).
local TURNED = 1
-- A panel a breath can hit: a solid one (0x080D3B78).
local SOLID = 0x10

-- 0x080D3EDC: an effect on the ground at (x, y)'s center, turned, not running while dimmed.
function breath.effect_at(x: number, y: number, effect: Effect): Object?
    local cx, cy = field.center(x, y)
    local o = battle.effect(Vec3.new(cx, cy, 0), effect, TURNED)
    if o then
        o.run_while_dimmed = false
    end
    return o
end

-- `fn` on each of the column ahead's hit panels: level, up, down.
function breath.on_column(me: Object, s: BreathState, fn: (x: number, y: number) -> ())
    local x, y = me.panel_x + me.facing, me.panel_y
    if s.ahead then
        fn(x, y)
    end
    if s.above then
        fn(x, y - 1)
    end
    if s.below then
        fn(x, y + 1)
    end
end

-- 0x080D3E74, 0x080D3EB0, 0x080D3EC6: a sound every 8 ticks, the count (+0x22) on from 0 (HeatBrth's, WoodPwdr's,
-- ElecShok's).
function breath.every_8(sound: Sound): (me: Object, s: BreathState) -> ()
    return function(_me: Object, s: BreathState)
        local n = s.count + 1
        if n >= 8 then
            battle.play_sound(sound)
            n = 0
        end
        s.count = n
    end
end

-- A chip's breath: its spawn (0x080D3B42), a breath on (x, y) from `user` with the attack's element and damage word.
-- (Its position is the spawner's registers until its init places it: Y, the element and 0.)
function breath.make(look: BreathLook): (user: Object, x: number, y: number, element: number, damage: number) -> ()
    -- 0x080D3B6C: on a solid panel, the effect, and a hit of the chip's element for 40 ticks at its height.
    local function hit(me: Object, x: number, y: number): boolean
        if not field.check(x, y, SOLID, 0) then
            return false
        end
        local fx = breath.effect_at(x, y, look.hit_effect)
        if fx and look.hit_effect_lasts then
            fx.timer = HIT_TICKS
        end
        local box = battle.hitbox(me, {
            panel_x = x,
            panel_y = y,
            element = look.element,
            z = me.pos.z,
            region = regions.single,
            hit_effect = look.spark,
            target = collision.hits_navis,
            self_type = collision.thrown,
            damage = me.damage,
            stamina = me.stamina,
            hit_mod = look.hit_mod,
            status = look.status,
        })
        if box then
            box.timer = HIT_TICKS
        end
        return true
    end

    -- 0x080D3DB6: the panel effect, and the panel's type.
    local function leave(x: number, y: number)
        if look.panel_effect then
            breath.effect_at(x, y, look.panel_effect)
        end
        field.set_type(x, y, look.panel)
    end

    -- 0x080D3DF8: 15 ticks before its end its own panel, on its last tick the column's.
    local function leaving(me: Object, s: BreathState)
        if me.timer == LEAVE_AT and s.hit then
            leave(me.panel_x, me.panel_y)
        end
        if me.timer == LAST then
            breath.on_column(me, s, leave)
        end
    end

    -- 0x080D3AD4: the battle over, gone; its chip's step and its panels left; its sound while its panel was hit;
    -- its time; 40 ticks in, the column ahead.
    local function tick(me: Object)
        local s = me.state :: BreathState
        if battle.over() then
            me:set_lifecycle("destroy")
            return
        end
        if look.step then
            look.step(me, s)
        end
        leaving(me, s)
        if s.hit then
            look.sound(me, s)
        end
        local t = me.timer - 1
        me.timer = int.u16(t)
        if t <= 0 then
            me:set_lifecycle("destroy")
            return
        end
        if t == COLUMN_AT and s.hit then
            local x, y = me.panel_x + me.facing, me.panel_y
            s.ahead = hit(me, x, y)
            s.above = hit(me, x, y - 1)
            s.below = hit(me, x, y + 1)
        end
    end

    -- 0x080D3AA0: its time; on its panel; its sound; its hit on its own panel; then its first tick.
    local function init(me: Object)
        me.timer = TICKS
        me:set_coordinates_from_panel()
        battle.play_sound(SOUND)
        local s = me.state :: BreathState
        s.hit = hit(me, me.panel_x, me.panel_y)
        me:set_lifecycle("update")
        tick(me)
    end

    local kind = new.kind {
        id = look.id,
        pool = "attack",
        state = { hit = "bool", ahead = "bool", above = "bool", below = "bool", count = "u16" },
        -- 0x080D3A7C: then the sprite.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me)
            elseif l == "update" then
                tick(me)
            else
                me:destroy()
            end
            me:update_sprite()
        end,
    }

    return function(user: Object, x: number, y: number, element: number, damage: number)
        local o = battle.spawn(kind, Vec3.new(y, element, 0))
        if o then
            o.panel_x, o.panel_y = x, y
            o.element = element
            o:set_damage_word(damage)
            o.alliance = user.alliance
        end
    end
end

-- The navi's number: what it holds animates by it (NaviStats +0x23, MegaMan's 0).
local NAVI_NUMBER = 0

local ANIM, HOLD_TICKS, OUT_AT, RECOVER_TICKS = 13, 36, 30, 5

type ActionState = { timer: number }

-- A chip's breath action: its key, its nozzle (its attachment row) and its breath's spawn.
function breath.action(spec: {
    id: string,
    nozzle: AttachmentLook,
    spawn: (user: Object, x: number, y: number, element: number, damage: number) -> (),
}): Action
    -- 0x080ED430: the nozzle up; 6 ticks in, its animation two on and the breath out; 37 ticks.
    local function hold(me: Object, s: ActionState)
        if me.step_init == 0 then
            me.anim = ANIM
            me:open_counter_window()
            attachment.attach(me, spec.nozzle, "related", 3 * NAVI_NUMBER)
            me:set_status("using_action", true)
            s.timer = HOLD_TICKS
            me.step_init = 4
        end
        if s.timer == OUT_AT then
            local nozzle = me.related1
            if nozzle then
                nozzle.anim += 2
            end
            local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
            spec.spawn(me, me.panel_x + me.facing, me.panel_y, me.attack_element, damage)
        end
        local t = s.timer - 1
        s.timer = int.u16(t)
        if t < 0 then
            me.step, me.step_init = 4, 0
        end
    end

    -- 0x080ED4A4: 6 ticks, then back to idle without the nozzle.
    local function recover(me: Object, s: ActionState)
        if me.step_init == 0 then
            s.timer = RECOVER_TICKS
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = int.u16(t)
        if t < 0 then
            me.related1 = nil
            me:exit_attack()
        end
    end

    return new.action {
        id = spec.id,
        state = { timer = "u16" },
        -- 0x080ED412.
        update = function(me: Object, s: ActionState)
            if me.step == 0 then
                hold(me, s)
            elseif me.step == 4 then
                recover(me, s)
            else
                error(string.format("a breath chip's phase %#x reads past its table (0x080ED428)", me.step))
            end
        end,
    }
end

return breath
