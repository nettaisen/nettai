--!strict
-- The flying shot (attack object #0xB, `sub_80C60A8`): TrnArrw's arrows,
-- the Beast forms' slash waves, a thrown obstacle. It flies forward at a
-- steady speed (and falls, if its third parameter says so) until it has
-- passed its range in panel centers, and ends on the first thing it hits
-- or when it leaves the field. Its kind, its first parameter, is data
-- (data.objects.flying_shots): its collision, element, spark, look,
-- speed, range and status. Its second parameter is how many ticks it
-- waits before it sets off; its fourth, its palette.
-- See docs/engine/objects-and-player.md §B8.

local element = require("../../lib/element")

export type State = {
    -- A thrown obstacle's kind and animation (kind | animation << 4), its
    -- spawner's word (the game's ExtraVars).
    obstacle: number,
}

local flying_shot = {
    state = { obstacle = "u8" },
}

-- How fast it falls when its third parameter says so, 16.16 a tick.
local FALL = -0x1_2000
-- A thrown obstacle with a sprite of this index keeps its parts' own
-- facing (`sub_8002EAC`), and is not flipped.
local UNFLIPPED_INDEX = 0x23
-- The height of a panel spark (`sub_801A100`), 16.16.
local PANEL_SPARK_Z = 0x10_0000
-- Collision results that show a hit spark (`object_spawnCollisionEffect`),
-- unless the first bit is set too.
local SPARKS, NO_SPARK = 0x3F800000, 1

-- `sub_80C6248`: a flying shot from `owner` at `pos`, with its parameters
-- (kind, ticks it waits, whether it falls, palette), the damage word
-- `damage` and, for a thrown obstacle, the obstacle (kind | animation <<
-- 4). It takes the owner's side and flip, and runs later this tick, right
-- after its spawner.
function flying_shot.spawn(owner: Object, pos: Vec3, params: { number }, damage: number, obstacle: number?): Object?
    local o = battle.spawn_kind("flying-shot", pos, params)
    if o then
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
        local s = o.state :: State
        s.obstacle = obstacle or 0
    end
    return o
end

-- Its kind's record (`byte_80C6038`): a kind past the table reads on into
-- the routine's code.
local function kind_of(me: Object): FlyingShotKind
    local n = me:param(1)
    local k = data.objects.flying_shots[n]
    if k == nil then
        error(string.format("flying shot kind %#x reads past byte_80C6038", n))
    end
    return k
end

-- Its end: not drawn, its hit region cleared (`object_clearCollisionRegion`);
-- the next update frees it.
local function finish(me: Object)
    me.visible = false
    me.collision.region = 0
    me:set_lifecycle("destroy")
end

-- `sub_800E6E8`: `c` lies past `a` and up to `b`, either way.
local function crossed(a: number, b: number, c: number): boolean
    if a <= b then
        return c > a and c <= b
    end
    return c > b and c <= a
end

-- `sub_801A100`: the hit spark over the center of the panel it is over
-- (`sub_800E258`), 16 pixels up (one RNG draw when it shows).
local function panel_spark(me: Object)
    local c = me.collision
    local f = c.hit_flags
    if not bit32.btest(f, SPARKS) or bit32.btest(f, NO_SPARK) or c.hit_effect == 0xFF then
        return
    end
    local p = me.pos
    local x = int.tdiv(int.asr(p.x, 16) + 0xA0, 0x28)
    local y = int.tdiv(int.asr(p.y, 16) + 0x20, 0x18)
    local cx, cy = field.center(x, y)
    battle.spark(me, battle.jitter(0xF, Vec3.new(cx, cy, PANEL_SPARK_Z)), c.hit_effect)
end

-- Fly (and fall); each panel center passed spends its range, and it ends
-- when that runs out (leaving its kind's end effect) or it leaves the
-- field.
local function fly(me: Object, k: FlyingShotKind)
    local p, v = me.pos, me.vel
    local cx = field.center(me.panel_x, me.panel_y)
    local x = int.i32(p.x + v.x)
    me.pos = Vec3.new(x, p.y, p.z + v.z)
    if crossed(x, p.x, cx) then
        local left = me:param(4) - 1
        me:set_param(4, left)
        if left <= 0 then
            local effect = k.end_effect
            if effect then
                me:set_coordinates_from_panel()
                battle.effect(me.pos, effect)
            end
            finish(me)
            return
        end
    end
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    if not field.valid(me.panel_x, me.panel_y) then
        finish(me)
    end
end

-- `sub_80C619C`: the hits against it resolve and spark; it ends on a hit,
-- else waits out its second parameter (with its kind's launch sound as
-- the wait ends) or flies. A highlighting kind lights its panel.
local function tick(me: Object)
    local k = kind_of(me)
    local c = me.collision
    c:remove()
    if k.panel_spark then
        panel_spark(me)
    else
        me:hit_spark()
    end
    if c.hit_flags ~= 0 then
        finish(me)
    else
        local wait = me:param(2)
        if wait ~= 0 then
            wait -= 1
            me:set_param(2, wait)
            local sound = k.launch_sound
            if wait == 0 and sound then
                battle.play_sound(sound)
            end
        else
            fly(me, k)
        end
    end
    c:present()
    -- `object_highlightCurrentCollisionPanels`: its region is its own
    -- panel (region 1) until it ends (region 0). Drawn only.
    if k.highlight and c.region ~= 0 then
        field.highlight(me.panel_x, me.panel_y)
    end
end

-- `sub_80C60CC`: its look, element, range, speed and collision from its
-- kind (a thrown obstacle's look from the obstacle), on the panel under
-- it. (The game overwrites its link to its owner with its record's
-- address, so the owner is forgotten.)
local function init(me: Object)
    local k = kind_of(me)
    me.related1 = nil
    local sprite: SpriteId = k.sprite
    local anim: number = k.anim or 0
    local s = me.state :: State
    if k.obstacle then
        local kind = bit32.band(s.obstacle, 0xF)
        local thrown = data.objects.absorbed_sprites[kind]
        if not thrown then
            error(string.format("absorbed obstacle kind %d is not in the content", kind))
        end
        sprite = thrown
        anim = bit32.band(bit32.rshift(s.obstacle, 4), 0xF)
    end
    local look = me.sprite
    look:load(sprite)
    look.shadow = if k.shadow then "ground" else "with_sprite"
    me.anim = anim
    me.anim_loaded = 0xFF
    me.visible = true
    -- The sprite's index: the part of "CC-II" after the dash.
    local index = tonumber(string.sub(sprite, 4), 16) or -1
    if k.obstacle and index == UNFLIPPED_INDEX then
        -- (`sub_8002EAC`: its parts keep their own facing; the sprite
        -- itself is not flipped. Drawn only.)
    else
        look:set_flip(bit32.bxor(me.alliance, me.flip))
    end
    look.palette = me:param(4)
    me.element = element.byte(k.element, k.secondary)
    me:set_param(4, k.range)
    me:set_panel_from_coordinates()
    me.vel = Vec3.new(me.facing * k.speed, me.vel.y, me.vel.z)
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(k.self_type, k.target_type, k.hit_mod)
    local c = me.collision
    c.hit_effect = k.hit_effect
    local status = k.status or 0
    if status ~= 0 then
        c.status_base = status
    end
    c:present()
    me.vel = Vec3.new(me.vel.x, me.vel.y, if me:param(3) ~= 0 then FALL else 0)
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80C60A8`.
function flying_shot.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
    me:update_sprite()
end

return flying_shot
