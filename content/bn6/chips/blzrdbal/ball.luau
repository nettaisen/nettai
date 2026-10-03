--!strict
-- BlzrdBal's snowball (the original's attack object #0xB2, `sub_80DB994`;
-- docs/engine/dimming-chip-effects.md §10): it rolls 3 pixels a tick to the
-- end of the field, hitting once in each column it enters, and stops on a
-- panel that isn't solid. Its roller (./roller), a step ahead, swallows the
-- field objects it meets: each makes the ball bigger and its hits harder.

local collision = require("../../rules/collision")
local regions = require("@common/regions")
local roller = require("./roller")

local ball = {}

export type State = {
    -- Its roller while it lives (ExtraVars+0; the roller clears it).
    roller: Object?,
    -- The damage each swallow adds (ExtraVars+4: the chip's damage, its
    -- low 12 bits).
    growth: number,
    -- Field objects swallowed (the original's first parameter).
    size: number,
}

-- What the roller tells it.
type RollerState = { swallowed: boolean }

local SPRITE = asset.sprite("bn6:blizzard-ball")
local SPEED = 0x3_0000
local SOUND_GROW = asset.sound("bn6:blizzard-grow")
-- What it is and reaches: an attack hitting navis, hit modifier 3, no
-- spark.
local IS, HITS, HIT_MOD = collision.attack, collision.hits_navis, 3
-- Its animation by size (`dword_80DBB08`): big after the first swallow.
local GROWN: { [number]: number } = { [1] = 1, [2] = 1 }
local SOLID = 0x10

-- `sub_80DBA7C`: grow if the roller swallowed something last tick; roll
-- on, hitting again in a new column; gone where the panel isn't solid or
-- off the field.
local function roll(me: Object)
    local s = me.state :: State
    local c = me.collision
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    local r = me.related2
    if s.roller and r and (r.state :: RollerState).swallowed then
        battle.play_sound(SOUND_GROW)
        c.self_damage = int.u16(c.self_damage + s.growth)
        s.size = int.u8(s.size + 1)
        if s.size < 3 then
            me.anim = GROWN[s.size]
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
local function tick(me: Object)
    me.collision:remove()
    me:hit_spark()
    local a = me.action
    if a ~= 0 then
        error(string.format("BlzrdBal's ball: action %#x reads past off_80DBA78", a))
    end
    roll(me)
    me.collision:present()
end

-- `sub_80DB9B8`: on the panel under it, small, with its roller; its
-- collision on no panel until it enters a column. (The update first runs
-- next tick.)
local function init(me: Object)
    me:set_panel_from_coordinates()
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.visible = true
    me.anim, me.anim_loaded = 0, 0
    sprite:set_animation(0)
    local s = me.state :: State
    s.size = 0
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    local r = roller.spawn(me)
    me.related2 = r
    if not r then
        me:free()
        return
    end
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(IS, HITS, HIT_MOD)
    me.vel = Vec3.new(SPEED * me.facing, me.vel.y, me.vel.z)
    me.collision:set_hit_effect(nil)
    me.collision:present()
    s.growth = bit32.band(me.damage, 0xFFF)
    me:set_lifecycle("update")
end

ball.kind = define.kind {
    id = "bn6:blzrdbal/ball",
    pool = "attack",
    state = { roller = "object", growth = "u16", size = "u8" },
    -- `sub_80DB994`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
            if not me.active then
                return
            end
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
            return
        end
        me:update_sprite()
    end,
}

-- `sub_80DBB0C`: a ball at (x, y) on the ground with `element` and the
-- damage word, of `owner`'s side and facing, running while dimmed.
function ball.spawn(owner: Object, x: number, y: number, element: number, damage: number): Object?
    local o = battle.spawn(ball.kind)
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
