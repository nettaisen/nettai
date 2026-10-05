--!strict
-- The yoyos: a yoyo (the original's attack object #0x52, `sub_80CE81C`),
-- YoYo's and each of GreatYo's three, and GreatYo's controller (effect
-- object #0x70, `sub_80E857C`), which sends three and keeps them.
--
-- A yoyo rolls out a panel per 8 ticks for its panels (5 pixels a tick,
-- re-arming its region on each new panel), spins there for its 8-tick
-- rounds (its region re-armed each round) and rolls back as far as it
-- went. A hit takes its region away until the next panel or round. It
-- runs while dimmed but only resolves its hits then. GreatYo's move
-- together: a hit on the way out sends all three into their spin at once
-- (`sub_80E8612`). When it goes it clears the slot that holds it: its
-- navi's first related object, or its controller's.
--
-- The controller is invisible. On its first update it sends out three
-- yoyos with its own spec and damage: on its panel's row below, above and
-- on it, and holds them. Once all three are gone it clears its navi's
-- first related object (which held it) and ends.

local collision = require("../../rules/collision")
local regions = require("@exelib/regions")

local yoyo = {}

-- Which slot holds a yoyo (ExtraVars+4, a pointer in the game): its
-- navi's first related object, or one of its GreatYo controller's.
export type YoyoSlot = "related" | "middle" | "below" | "above"

-- How a yoyo flies (`YoyoSpec`, types.d.luau): GreatYo's (which move
-- together, the original's second parameter), the panels it rolls out
-- (the third) and its spinning rounds (the fourth).
export type Spec = YoyoSpec

export type State = {
    great: boolean,
    panels: number,
    rounds: number,
    -- Panels or rounds left (ExtraVars+0).
    left: number,
    slot: YoyoSlot,
    -- 1 once it hit something (and when told to spin by another yoyo; 2
    -- makes it skip its next spin tick) (ExtraVars+8).
    hit: number,
    -- Panels to roll back (ExtraVars+0xC).
    back: number,
}

-- The controller's state.
export type ControllerState = {
    great: boolean,
    panels: number,
    rounds: number,
    -- The yoyos it holds (ExtraVars+0, +4 and +8); the yoyos clear these
    -- when they go.
    middle: Object?,
    below: Object?,
    above: Object?,
}

local SPRITE = asset.sprite("yoyo")
local SOUND = asset.sound("set-down")
local HIT_MOD = 1
-- Its speed, 16.16 per tick, and the ticks per panel or round.
local SPEED, ROUND = 0x5_0000, 8
-- The actions: out, spinning, back.
local OUT, SPIN, BACK = 0, 4, 8

-- The controller that holds a GreatYo's yoyo (its first related object).
local function controller(me: Object): Object?
    local owner = me.related1
    if owner == nil then
        return nil
    end
    if owner.kind ~= yoyo.controller then
        error("a GreatYo's yoyo reads a navi's ExtraVars as a GreatYo controller (sub_80E8612)")
    end
    return owner
end

-- `sub_80CEAC2`: tell `target` to spin (snapped to its panel), with the
-- hit flag `hit` (1 for the caller itself, and every yoyo after it) and
-- `back` panels to roll back. Returns the flag the next yoyo gets.
local function signal(me: Object, target: Object?, hit: number, back: number): number
    if target == me then
        hit = 1
    end
    if target then
        target:set_coordinates_from_panel()
        local s = target.state :: State
        s.hit = hit
        s.back = back
        target:set_action(SPIN)
    end
    return hit
end

-- `sub_80E8612`: all three of the GreatYo's yoyos spin, in the
-- controller's order.
local function all_spin(me: Object, back: number)
    local c = controller(me)
    if c == nil then
        return
    end
    local slots = c.state :: ControllerState
    local hit = 2
    hit = signal(me, slots.below, hit, back)
    hit = signal(me, slots.above, hit, back)
    signal(me, slots.middle, hit, back)
end

-- `sub_800E258`'s column: the panel under X.
local function column(x: number): number
    return int.tdiv(int.asr(x, 16) + 0xA0, 0x28)
end

-- Roll a tick; on a new panel its region is back.
local function roll(me: Object)
    local p = me.pos
    local x = int.i32(p.x + me.vel.x)
    me.pos = Vec3.new(x, p.y, p.z)
    local col = column(x)
    if col ~= me.panel_x then
        me.panel_x = col
        me.collision.panel_x = col
        me.collision:set_region(regions.single)
    end
end

-- Out of panels (or a GreatYo's hit): spin, with the panels to roll back.
local function turn(me: Object, s: State)
    local back = s.panels - s.left
    if s.great then
        all_spin(me, back)
    else
        s.back = back
        me:set_action(SPIN)
    end
end

-- `sub_80CE8D8`: roll out.
local function out(me: Object, s: State)
    local phase = me.phase
    if phase == 0 then
        -- sub_80CE8F4
        me.timer = ROUND
        s.left = s.panels
        me.phase, me.phase_init = 4, 0
        if s.great and s.hit ~= 0 then
            s.back = s.panels - s.left
            me:set_action(SPIN)
            all_spin(me, 0)
        end
    elseif phase == 4 then
        -- sub_80CE932
        local t = me.timer - 1
        me.timer = t
        if t ~= 0 then
            roll(me)
            return
        end
        me:set_coordinates_from_panel()
        local left = s.left - 1
        s.left = left
        if left <= 0 or (s.great and s.hit ~= 0) then
            turn(me, s)
            return
        end
        me.timer = ROUND
    else
        error(string.format("yoyo phase %#x reads past off_80CE8EC", phase))
    end
end

-- `sub_80CE9A4`: spin for its rounds.
local function spin(me: Object, s: State)
    if s.hit == 2 then
        s.hit = 1
        return
    end
    local phase = me.phase
    if phase == 0 then
        -- sub_80CE9CC
        me.timer = ROUND
        s.left = s.rounds
        me.phase, me.phase_init = 4, 0
    elseif phase == 4 then
        -- sub_80CE9DC
        local t = me.timer - 1
        me.timer = t
        if t ~= 0 then
            return
        end
        me.collision:set_region(regions.single)
        local left = s.left - 1
        s.left = left
        if left ~= 0 then
            me.timer = ROUND
            return
        end
        me:set_action(BACK)
        local v = me.vel
        me.vel = Vec3.new(-v.x, v.y, v.z)
    else
        error(string.format("yoyo phase %#x reads past off_80CE9C4", phase))
    end
end

-- Home: the destroy state.
local function home(me: Object)
    me:set_lifecycle("destroy")
    me.collision:set_region(nil)
end

-- `sub_80CEA0A`: roll back as far as it went.
local function back(me: Object, s: State)
    local phase = me.phase
    if phase == 0 then
        -- sub_80CEA24
        me.timer = ROUND
        s.left = s.back
        me.phase, me.phase_init = 4, 0
        if s.left == 0 then
            home(me)
        end
    elseif phase == 4 then
        -- sub_80CEA42
        local t = me.timer - 1
        me.timer = t
        if t ~= 0 then
            roll(me)
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
        error(string.format("yoyo phase %#x reads past off_80CEA1C", phase))
    end
end

-- `sub_80CE87C`: resolve its hits; move unless dimmed.
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
            error(string.format("yoyo action %#x reads past off_80CE8CC", a))
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

-- `sub_8011504`: the sprite with a ground shadow, then collision (false
-- when no slot was free).
local function setup(me: Object): boolean
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite:set_animation(0)
    sprite.shadow = "ground"
    me.anim = 0
    me.anim_loaded = 0xFF
    if not me:create_collision() then
        return false
    end
    me:setup_collision(collision.slash, collision.hits_navis, HIT_MOD)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    return true
end

-- `sub_80CE83C`.
local function init(me: Object)
    me.visible = true
    if not setup(me) then
        me:free()
        return
    end
    me:set_coordinates_from_panel()
    local v = me.vel
    me.vel = Vec3.new(me.facing * SPEED, v.y, v.z)
    me.collision:present()
    battle.play_sound(SOUND)
    me.lifecycle = "update"
end

-- `sub_80CEA88`: clear the slot that holds it (if it still does), then
-- free it and its collision.
local function gone(me: Object)
    local s = me.state :: State
    local owner = me.related1
    if owner then
        if s.slot == "related" then
            if owner.related1 == me then
                owner.related1 = nil
            end
        else
            local slots = owner.state :: ControllerState
            local held: Object? = if s.slot == "middle"
                then slots.middle
                elseif s.slot == "below" then slots.below
                else slots.above
            if held == me then
                if s.slot == "middle" then
                    slots.middle = nil
                elseif s.slot == "below" then
                    slots.below = nil
                else
                    slots.above = nil
                end
            end
        end
    end
    me.collision:free()
    me:free()
end

yoyo.kind = define.kind {
    id = "yoyo/yoyo",
    pool = "attack",
    state = {
        great = "bool",
        panels = "u8",
        rounds = "u8",
        left = "i32",
        slot = { "related", "middle", "below", "above" },
        hit = "u32",
        back = "i32",
    },
    -- `sub_80CE81C`.
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

-- `sub_80CEAA2`: a yoyo for `owner` on (x, y) at height `z`, with the
-- damage word `damage`, flying as `spec` says, held in `slot`. It runs
-- while dimmed. Its position is the spawner's registers (the panel Y, the
-- element and the height) until its init places it.
function yoyo.spawn(
    owner: Object,
    x: number,
    y: number,
    element: number,
    z: number,
    damage: number,
    spec: Spec,
    slot: YoyoSlot
): Object?
    local o = battle.spawn(yoyo.kind, Vec3.new(y, element, z))
    if o then
        local s = o.state :: State
        s.great, s.panels, s.rounds = spec.great or false, spec.panels, spec.rounds
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
        s.slot = slot
        o.run_while_dimmed = true
    end
    return o
end

-- `sub_80E862E`: one of the controller's yoyos, on row `y`, held in
-- `slot`.
local function send(me: Object, y: number, slot: "middle" | "below" | "above"): Object?
    local c = me.state :: ControllerState
    local damage = me.damage + me.stamina * 0x10000
    local spec: YoyoSpec = { great = c.great, panels = c.panels, rounds = c.rounds }
    return yoyo.spawn(me, me.panel_x, y, me.element, me.pos.z, damage, spec, slot)
end

-- `sub_80E85A8`: send the yoyos; end once they're all gone.
local function control(me: Object)
    local s = me.state :: ControllerState
    if me.phase_init == 0 then
        local y = me.panel_y
        s.below = send(me, y + 1, "below")
        s.above = send(me, y - 1, "above")
        s.middle = send(me, y, "middle")
        me.phase_init = 4
    end
    if s.middle == nil and s.below == nil and s.above == nil then
        local navi = me.related1
        if navi then
            navi.related1 = nil
        end
        me:set_lifecycle("destroy")
    end
end

yoyo.controller = define.kind {
    id = "yoyo/great-yoyo",
    pool = "effect",
    state = {
        great = "bool",
        panels = "u8",
        rounds = "u8",
        middle = "object",
        below = "object",
        above = "object",
    },
    -- `sub_80E857C`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- sub_80E859C
            me:set_lifecycle("update")
            control(me)
        elseif l == "update" then
            control(me)
        else
            me:free()
        end
    end,
}

-- `sub_80E85FA`: a controller for `owner` on (x, y) at height `z` with the
-- damage word `damage`, its yoyos flying as `spec` says. Its first related
-- object is the navi, whose first related object holds it. Its position
-- stays the spawner's registers (the panel Y, the element and the
-- height).
function yoyo.spawn_controller(
    owner: Object,
    x: number,
    y: number,
    element: number,
    z: number,
    damage: number,
    spec: Spec
): Object?
    local o = battle.spawn(yoyo.controller, Vec3.new(y, element, z))
    if o then
        local s = o.state :: ControllerState
        s.great, s.panels, s.rounds = spec.great or false, spec.panels, spec.rounds
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
    end
    return o
end

return yoyo
