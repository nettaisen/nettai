--!strict
-- CopyDmg's mark (attack object #0x28, `sub_80C8FFC`): an unseen probe that
-- steps a panel a tick along its row until it hits something. If that was
-- a navi (a player on the panel, found in the other side's actor list), it
-- marks the navi for 180 ticks: the navi's side's damage-carry record
-- makes it take, each tick, the damage its side took the tick before on
-- top of its own (the intake's `sub_802CE10`). The mark shows over the
-- navi (at its attach point 0x1B) as long as the navi is drawn, and goes
-- when its time is up or the record is no longer its. A probe that hits
-- nothing, or leaves the field, or finds time up, just goes.
--
-- Four states: init, flying (update), marking (destroy) and gone
-- (finish).

local copy_mark = {}

local SPRITE = "14-05"
local ANIM = 0x0B
-- How long a navi stays marked.
local MARK_TICKS = 0xB4
-- The attach point the mark sits at, on the marked navi.
local MARK_POINT = 0x1B
-- A panel with a player navi on it (either side's player bit).
local PLAYERS = 0x60_0000

-- `sub_80C9184`: a probe on (x, y) for `owner`, with Param1..4 `params` and
-- the damage word `damage`. It goes at the end of the update list
-- (`sub_8003374`). Its position is the spawner's registers (the panel Y,
-- the element and 0) until its init places it.
function copy_mark.spawn(owner: Object, x: number, y: number, element: number, params: { number }, damage: number): Object?
    local o = battle.spawn_kind_at_end("copy-mark", Vec3.new(y, element, 0), params)
    if o then
        o.panel_x, o.panel_y = x, y
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.alliance, o.flip = owner.alliance, owner.flip
    end
    return o
end

-- The damage-carry record the probe's side sets: the other side's (the
-- one that takes the damage).
local function carry_side(me: Object): number
    return if me.alliance == 0 then 1 else 0
end

-- `sub_800AE54`: the first of `side`'s actors standing on (x, y).
local function actor_at(x: number, y: number, side: number): Object?
    for _, o in battle.alive_actors(side) do
        if o.panel_x == x and o.panel_y == y then
            return o
        end
    end
    return nil
end

-- `sub_80C913C`: mark the navi on the probe's panel, if a player stands
-- there: it becomes the damage-carry record's target, with the probe as
-- its source.
local function mark(me: Object): boolean
    local x, y = me.panel_x, me.panel_y
    if not bit32.btest(field.flags(x, y), PLAYERS) then
        return false
    end
    local target = actor_at(x, y, 1 - me.alliance)
    if target == nil then
        return false
    end
    me.related1 = target
    battle.set_damage_carry(carry_side(me), { this_tick = 0, previous = 0, source = me, target = target })
    return true
end

-- Gone: hidden, then freed.
local function vanish(me: Object)
    me.visible = false
    me:set_lifecycle("finish")
end

-- `sub_80C9074`: resolve its hits; on a hit, mark (or go); else a panel on.
local function fly(me: Object)
    local c = me.collision
    c:remove()
    if c.hit_flags ~= 0 then
        battle.play_sound(0xBD)
        c.region = 0
        if mark(me) then
            me.anim = ANIM
            me.anim_loaded = 0xFF
            me.timer = MARK_TICKS
            me:set_lifecycle("destroy")
            return
        end
        return vanish(me)
    end
    if battle.time_up() then
        return vanish(me)
    end
    -- Its X velocity is a panel step (±1).
    me.panel_x = me.panel_x + me.vel.x
    me:set_coordinates_from_panel()
    me:update_collision_panels()
    if not field.valid(me.panel_x, me.panel_y) then
        return vanish(me)
    end
    c:present()
end

-- `sub_80C90DC`: over the marked navi while the record is the probe's,
-- for its time; then the record forgets it (`sub_802CDE6`) and it goes.
local function marking(me: Object)
    local target = me.related1 :: Object
    me.visible = target.visible
    local side = carry_side(me)
    local rec = battle.damage_carry(side)
    if rec.source == me and rec.target ~= nil then
        -- The attach point as the navi's table has it (the API's faces
        -- the navi's way), mirrored the probe's way. (For NameIDs 0xCD to
        -- 0xFF the game would read another table; only navis get marked.)
        local ax, ay = target:attach_point(MARK_POINT)
        ax *= target.facing
        local dx = int.mul32(ax * 0x10000, -me.facing)
        me.pos = target.pos + Vec3.new(dx, 0, ay * 0x10000)
        local t = me.timer - 1
        me.timer = t
        if t ~= 0 then
            return
        end
    end
    if rec.source == me then
        battle.set_damage_carry(side, { this_tick = rec.this_tick, previous = rec.previous, source = nil, target = nil })
    end
    vanish(me)
end

-- `sub_80C9024`.
local function init(me: Object)
    local sprite = me.sprite
    sprite:load(SPRITE)
    me.anim = ANIM
    sprite:set_animation(ANIM)
    sprite.shadow = "with_sprite"
    sprite:step()
    me:set_coordinates_from_panel()
    me.vel = Vec3.new(me.facing, me.vel.y, me.vel.z)
    me.element = 5
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(4, 5, 0)
    me.collision:present()
    me:set_lifecycle("update")
end

-- `sub_80C8FFC`.
function copy_mark.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
        if not me.active then
            return
        end
    elseif l == "update" then
        fly(me)
    elseif l == "destroy" then
        marking(me)
    else
        me:destroy()
        return
    end
    me:update_sprite()
end

return copy_mark
