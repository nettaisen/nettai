--!strict
-- The snowball, as EXE6 and EXE5 share it (EXE6's attack object #0xB2,
-- `sub_80DB994`, BlzrdBal's; EXE5's #0xB2, 0x080DC880, BlizMan's;
-- docs/engine/dimming-chip-effects.md §10): it rolls 3 pixels a tick to
-- the end of the field, hitting once in each column it enters, and stops
-- on a panel that isn't solid. Its roller (./roller), a step ahead,
-- swallows the field objects it meets: each makes the ball bigger and its
-- hits harder. Each game makes its own kind (`ball.make_kind`: its sprite
-- and animations, its roller, how much harder each swallow makes it, and
-- the sound it makes).

local regions = require("../regions")
local roller = require("./roller")

local ball = {}

export type State = {
    -- Its roller while it lives (ExtraVars+0; the roller clears it).
    roller: Object?,
    -- The damage each swallow adds (ExtraVars+4).
    growth: number,
    -- Field objects swallowed (the original's first parameter).
    size: number,
}

-- A game's snowball: its kind's key, its sprite, its animation and its
-- animations grown (by size, 1 and 2: `dword_80DBB08`), its roller's
-- kind, what it is and reaches, what each swallow adds to its hits (a
-- number, or "damage": its damage's low 12 bits), and its sound then.
export type Look = {
    id: string,
    sprite: SpriteAsset,
    anim: number,
    grown: { number },
    roller: Kind,
    self_type: CollisionType,
    target: CollisionType,
    growth: number | "damage",
    grow_sound: Sound?,
}

-- What the roller tells it.
type RollerState = { swallowed: boolean }

local SPEED = 0x3_0000
-- Its hit modifier, no spark.
local HIT_MOD = 3
local SOLID = 0x10

-- `sub_80DBA7C`: grow if the roller swallowed something last tick; roll
-- on, hitting again in a new column; gone where the panel isn't solid or
-- off the field.
local function roll(me: Object, look: Look)
    local s = me.state :: State
    local c = me.collision
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    local r = me.related2
    if s.roller and r and (r.state :: RollerState).swallowed then
        if look.grow_sound then
            battle.play_sound(look.grow_sound)
        end
        c.self_damage = int.u16(c.self_damage + s.growth)
        s.size = int.u8(s.size + 1)
        if s.size < 3 then
            me.anim = look.grown[s.size]
        end
    end
    local x = me.panel_x
    me.pos = Vec3.new(me.pos.x + me.vel.x, me.pos.y, me.pos.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    if me.panel_x ~= x then
        c:set_region(regions.single)
    end
    if not bit32.btest(field.flags(me.panel_x, me.panel_y), SOLID) or not field.valid(me.panel_x, me.panel_y) then
        c:set_region(nil)
        me:set_lifecycle("destroy")
    end
end

-- `sub_80DBA50`: resolve last tick's hits, roll, present.
local function tick(me: Object, look: Look)
    me.collision:remove()
    me:hit_spark()
    local a = me.action
    if a ~= 0 then
        error(string.format("the snowball: action %#x reads past off_80DBA78", a))
    end
    roll(me, look)
    me.collision:present()
end

-- `sub_80DB9B8`: on the panel under it, small, with its roller; its
-- collision on no panel until it enters a column. (The update first runs
-- next tick.)
local function init(me: Object, look: Look)
    me:set_panel_from_coordinates()
    local sprite = me.sprite
    sprite:load(look.sprite)
    sprite.shadow = "ground"
    me.visible = true
    me.anim, me.anim_loaded = look.anim, look.anim
    sprite:set_animation(look.anim)
    local s = me.state :: State
    s.size = 0
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    local r = roller.spawn(look.roller, me)
    me.related2 = r
    if not r then
        me:free()
        return
    end
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(look.self_type, look.target, HIT_MOD)
    me.vel = Vec3.new(SPEED * me.facing, me.vel.y, me.vel.z)
    me.collision:set_hit_effect(nil)
    me.collision:present()
    s.growth = if look.growth == "damage" then bit32.band(me.damage, 0xFFF) else look.growth :: number
    me:set_lifecycle("update")
end

-- A game's snowball kind.
function ball.make_kind(look: Look): Kind
    return define.kind {
        id = look.id,
        pool = "attack",
        state = { roller = "object", growth = "u16", size = "u8" },
        -- `sub_80DB994`.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me, look)
                if not me.active then
                    return
                end
            elseif l == "update" then
                tick(me, look)
            else
                me:destroy()
                return
            end
            me:update_sprite()
        end,
    }
end

-- `sub_80DBB0C`: a ball (of `kind`) at (x, y) on the ground with
-- `element` and the damage word, of `owner`'s side and facing, running
-- while dimmed.
function ball.spawn(kind: Kind, owner: Object, x: number, y: number, element: number, damage: number): Object?
    local o = battle.spawn(kind)
    if not o then
        return nil
    end
    o.pos = Vec3.new(x, y, 0)
    o.element = element
    o:set_damage_word(damage)
    o.alliance, o.flip = owner.alliance, owner.flip
    o.run_while_dimmed = true
    return o
end

return ball
