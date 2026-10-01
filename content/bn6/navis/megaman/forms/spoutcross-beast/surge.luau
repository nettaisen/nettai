--!strict
-- A surge of water (the original's attack object #0x76, `sub_80D385C`),
-- SpoutCross Beast's charged chip's (./charge): it stands 20 pixels ahead
-- of its panel, on the ground, and hits its panel and the one ahead for
-- its ticks; a hit takes its region away until its next beat. It ebbs
-- (animation 1) 6 ticks in and falls (animation 2) with 8 left.

local collision = require("../../../../rules/collision")
local regions = require("../../../../lib/regions")

local surge = {}

-- What a surge is spawned with: how long it lasts (the original's first
-- parameter) and how often its region comes back (its second), in ticks.
type Spec = AquaSurge

export type State = { ticks: number, beat: number }

local SPRITE = asset.sprite("aqua-surge")
local SOUND = asset.sound("aqua-surge")

-- How far ahead of its panel's center it stands, 16.16.
local AHEAD = 0x14_0000

-- Its hits: an attack at navis, on its panel and the one ahead, no spark.
local HIT_MOD = 1
local REGION = regions.long

-- The end: no region, hidden, destroyed.
local function finish(me: Object)
    me.collision:set_region(nil)
    me.visible = false
    me:set_lifecycle("destroy")
end

-- `sub_80D3946`: the animation by the ticks left; the region back every
-- beat; the end after its ticks.
local function count(me: Object, s: State)
    if me.timer == s.ticks - 6 then
        me.anim = 1
    end
    if me.timer == 8 then
        me.anim = 2
    end
    local t = me.timer - 1
    me.timer = t
    if t == 0 then
        return finish(me)
    end
    local t2 = me.timer2 - 1
    me.timer2 = t2
    if t2 > 0 then
        return
    end
    me.timer2 = s.beat
    me.collision:set_region(REGION)
end

-- `sub_80D390A`: resolve the hits (a hit takes the region away), count,
-- and present again (even on its last tick); gone once the battle is over.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        me.visible = false
        c:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    count(me, me.state :: State)
    me:update_sprite()
    c:present()
end

-- `sub_80D387C`: ahead of its panel (not off the field), with its
-- collision, and a splash.
local function init(me: Object)
    if not field.valid(me.panel_x, me.panel_y) then
        me:free()
        return
    end
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "hidden"
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x + me.facing * AHEAD, p.y, 0)
    sprite.palette = 0
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.attack, collision.hits_navis, HIT_MOD)
    local c = me.collision
    c:set_hit_effect(nil)
    me.timer2 = (me.state :: State).beat
    c:set_region(REGION)
    c:present()
    battle.play_sound(SOUND)
    me:set_lifecycle("update")
    tick(me)
end

surge.kind = define.kind {
    id = "spoutcross-beast/surge",
    pool = "attack",
    state = { ticks = "u8", beat = "u8" },
    -- `sub_80D385C`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
    end,
}

-- `sub_80D398E`: a surge from `owner` on (x, y) with `element` and the
-- damage word. It takes the owner's side (not its facing). Its position is
-- the registers (y, the element, 0) until its init places it.
function surge.spawn(owner: Object, x: number, y: number, element: number, spec: Spec, damage: number): Object?
    local o = battle.spawn(surge.kind, Vec3.new(y, element, 0))
    if o then
        local s = o.state :: State
        s.ticks, s.beat = spec.ticks, spec.beat
        o.element = element
        o.panel_x, o.panel_y = x, y
        o.timer = spec.ticks
        o:set_damage_word(damage)
        o.alliance = owner.alliance
    end
    return o
end

return surge
