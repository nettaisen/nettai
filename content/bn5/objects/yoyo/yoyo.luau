--!strict
-- BN5's yoyo (attack object #0x52, 0x080D00A8: BN6's number, an earlier form of its code) and the YoYo chips'
-- throw (BN6's, the shared @common/yoyo/throw, with BN5's yoyo arm: the attachment table's row 0x26).
--
-- A yoyo rolls out a panel per 8 ticks for its panels (5 pixels a tick), spins there for its 8-tick rounds and
-- rolls back as far as it went; on each new panel and each round its region is re-armed (YoYo's: the panel; a
-- GreatYo leader's: a column of three; a GreatYo follower's: none, its region off from the start). A hit takes its
-- region away until then. It moves only while the battle isn't dimmed. A GreatYo yoyo (its Param2 1 or 2) turns at
-- its next panel once it has hit, and the leader's hit (or turn) tells the other two (0x080E8A5A: each yoyo its
-- controller holds is marked as having hit). When it goes it clears the slot that holds it, if that still does.
--
-- Unlike BN6's: its collision is row 0x40 (BN6's slash), no snapping to the panel on another yoyo's hit, and no
-- skipped spin tick.

local collision = require("../../rules/collision")
local regions = require("@common/regions")
local throw = require("@common/yoyo/throw")
local attachment = require("../attachment/attachment")

local yoyo = {}

-- How a yoyo flies (its Param2 to Param4): YoYo's (0), a GreatYo leader's (1) or follower's (2), the panels it rolls
-- out and its spinning rounds.
export type Spec = { mode: number, panels: number, rounds: number }

-- Which slot holds it (ExtraVars+4, a pointer in the game): its thrower's first related object, or one of its
-- controller's (ExtraVars+0, +4, +8: `middle`, `below`, `above`).
export type Slot = "related" | "middle" | "below" | "above"

export type State = {
    spec: Spec,
    slot: Slot,
    -- Panels or rounds left (ExtraVars+0).
    left: number,
    -- It hit something, or was told so (ExtraVars+8).
    hit: number,
    -- Panels to roll back (ExtraVars+0xC), and that it turned (ExtraVars+0x10).
    back: number,
    turned: number,
}

local SPRITE = asset.sprite("bn5:yoyo")
local SOUND = asset.sound("bn5:set-down")
local HIT_MOD = 1
local SPEED, ROUND = 0x5_0000, 8
local OUT, SPIN, BACK = 0, 4, 8
local PLAIN, LEADER, FOLLOWER = 0, 1, 2

-- Its region by its mode (none for a follower).
local function arm(me: Object, s: State)
    local mode = s.spec.mode
    if mode == FOLLOWER then
        return
    end
    me.collision:set_region(if mode == PLAIN then regions.single else regions.wide)
end

-- 0x080E8A5A: the controller's three yoyos are told they hit.
local function tell_all(me: Object)
    local c = me.related1
    if c == nil then
        return
    end
    local slots = c.state :: { middle: Object?, below: Object?, above: Object? }
    for _, o in { slots.middle, slots.below, slots.above } do
        if o then
            (o.state :: State).hit = 1
        end
    end
end

-- `sub_800E258`'s column: the panel under X.
local function column(x: number): number
    return int.tdiv(int.asr(x, 16) + 0xA0, 0x28)
end

-- Roll a tick; on a new panel its region is back.
local function roll(me: Object, s: State)
    local p = me.pos
    local x = int.i32(p.x + me.vel.x)
    me.pos = Vec3.new(x, p.y, p.z)
    local col = column(x)
    if col ~= me.panel_x then
        me.panel_x = col
        me.collision.panel_x = col
        arm(me, s)
    end
end

local function set_action(me: Object, a: number)
    me.action, me.phase, me.phase_init = a, 0, 0
end

-- 0x080D021C: spin, with the panels to roll back; a leader tells the others.
local function turn(me: Object, s: State)
    s.back = s.spec.panels - s.left
    s.turned = 1
    set_action(me, SPIN)
    if s.spec.mode == LEADER then
        tell_all(me)
    end
end

-- 0x080D0170: roll out.
local function out(me: Object, s: State)
    local phase = me.phase
    if phase == 0 then
        -- 0x080D018C
        me.timer = ROUND
        s.left = s.spec.panels
        me.phase, me.phase_init = 4, 0
        if s.spec.mode ~= PLAIN and s.hit ~= 0 then
            turn(me, s)
        end
    elseif phase == 4 then
        -- 0x080D01C8
        local t = me.timer - 1
        me.timer = t
        if t ~= 0 then
            roll(me, s)
            return
        end
        me:set_coordinates_from_panel()
        local left = s.left - 1
        s.left = left
        if left <= 0 or (s.spec.mode ~= PLAIN and s.hit ~= 0) then
            turn(me, s)
            return
        end
        me.timer = ROUND
    else
        error(string.format("yoyo phase %#x reads past 0x080D0184", phase))
    end
end

-- 0x080D023E: spin for its rounds.
local function spin(me: Object, s: State)
    local phase = me.phase
    if phase == 0 then
        -- 0x080D0258
        me.timer = ROUND
        s.left = s.spec.rounds
        me.phase, me.phase_init = 4, 0
    elseif phase == 4 then
        -- 0x080D0268
        local t = me.timer - 1
        me.timer = t
        if t ~= 0 then
            return
        end
        arm(me, s)
        local left = s.left - 1
        s.left = left
        if left ~= 0 then
            me.timer = ROUND
            return
        end
        set_action(me, BACK)
        local v = me.vel
        me.vel = Vec3.new(-v.x, v.y, v.z)
    else
        error(string.format("yoyo phase %#x reads past 0x080D0250", phase))
    end
end

local function home(me: Object)
    me:set_lifecycle("destroy")
    me.collision:set_region(nil)
end

-- 0x080D02A2: roll back as far as it went.
local function back(me: Object, s: State)
    local phase = me.phase
    if phase == 0 then
        -- 0x080D02BC
        me.timer = ROUND
        s.left = s.back
        me.phase, me.phase_init = 4, 0
        if s.left == 0 then
            home(me)
        end
    elseif phase == 4 then
        -- 0x080D02DA
        local t = me.timer - 1
        me.timer = t
        if t ~= 0 then
            roll(me, s)
            return
        end
        me:set_coordinates_from_panel()
        local left = s.left - 1
        s.left = left
        if left <= 0 then
            home(me)
            return
        end
        me.timer = ROUND
    else
        error(string.format("yoyo phase %#x reads past 0x080D02B4", phase))
    end
end

-- 0x080D0114: resolve its hits; move unless dimmed.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    local s = me.state :: State
    if c.hit_flags ~= 0 then
        s.hit = 1
        c:set_region(nil)
    end
    local dimmed = battle.dimmed()
    if not dimmed then
        local a = me.action
        if a == OUT then
            out(me, s)
        elseif a == SPIN then
            spin(me, s)
        elseif a == BACK then
            back(me, s)
        else
            error(string.format("yoyo action %#x reads past 0x080D0164", a))
        end
    end
    if c.region ~= nil then
        field.highlight(me.panel_x, me.panel_y)
    end
    if not dimmed then
        me:update_sprite()
    end
    c:present()
end

-- 0x080D00C8.
local function init(me: Object)
    me.visible = true
    -- sub_8011504: the sprite with a ground shadow, then collision.
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite:set_animation(0)
    sprite.shadow = "ground"
    me.anim, me.anim_loaded = 0, 0xFF
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.collision_40, collision.hits_navis, HIT_MOD)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    local v = me.vel
    me.vel = Vec3.new(me.facing * SPEED, v.y, v.z)
    if (me.state :: State).spec.mode == FOLLOWER then
        me.collision:set_region(nil)
    end
    me.collision:present()
    battle.play_sound(SOUND)
    me.lifecycle = "update"
end

-- 0x080D032C: clear the slot that holds it (if it still does), then free it and its collision.
local function gone(me: Object)
    local s = me.state :: State
    local owner = me.related1
    if owner then
        if s.slot == "related" then
            if owner.related1 == me then
                owner.related1 = nil
            end
        else
            local slots = owner.state :: { middle: Object?, below: Object?, above: Object? }
            if s.slot == "middle" and slots.middle == me then
                slots.middle = nil
            elseif s.slot == "below" and slots.below == me then
                slots.below = nil
            elseif s.slot == "above" and slots.above == me then
                slots.above = nil
            end
        end
    end
    me.collision:free()
    me:free()
end

yoyo.kind = define.kind {
    id = "bn5:yoyo/yoyo",
    pool = "attack",
    state = {
        spec = "record:bn5-yoyo",
        slot = { "related", "middle", "below", "above" },
        left = "i32",
        hit = "u32",
        back = "i32",
        turned = "u32",
    },
    -- 0x080D00A8.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            gone(me)
        end
    end,
}

function yoyo.spec(spec: Spec): Spec
    return define.record("bn5-yoyo", spec)
end

-- 0x080D0346: a yoyo for `owner` on (x, y) at height `z`, with `element` and the damage word, flying as `spec`
-- says, held in `slot`. It runs while dimmed. (Its position is the spawner's registers until its init places it.)
function yoyo.spawn(owner: Object, x: number, y: number, element: number, z: number, damage: number, spec: Spec, slot: Slot): Object?
    local o = battle.spawn(yoyo.kind, Vec3.new(y, element, z))
    if o then
        local s = o.state :: State
        s.spec, s.slot = spec, slot
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
        o.run_while_dimmed = true
    end
    return o
end

-- The YoYo chips' yoyo arm (row 0x26).
yoyo.arm = attachment.look { sprite = asset.sprite("bn5:yoyo-arm"), palette = 0, attach_point = 0x0F }

-- YoYo's throw: a yoyo of `spec`.
function yoyo.action(id: string, spec: Spec): Action
    return throw.action {
        id = id,
        arm = yoyo.arm,
        attachment = attachment.kind,
        throw = function(me: Object, x: number, y: number, z: number, damage: number): Object?
            return yoyo.spawn(me, x, y, 0, z, damage, spec, "related")
        end,
    }
end

return yoyo
